use axum::{
    extract::{BodyStream, ContentLengthLimit, Path},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use bytes::Bytes;
use chrono::Utc;
use diesel::prelude::*;
use futures::{AsyncWrite, AsyncWriteExt, TryStreamExt};
use imageinfo::{ImageFormat, ImageInfo, ImageInfoError};
use serde_json::json;

use pic_store_db as db;
use pic_store_storage as storage;

use db::{
    object_id::{BaseImageId, StorageLocationId},
    Permission,
};

use crate::{auth::UserInfo, shared_state::State, Error};

struct Header {
    buf: HeaderBuf,
}

impl Header {
    fn new() -> Header {
        Header {
            buf: HeaderBuf::Empty,
        }
    }
}

enum HeaderBuf {
    Empty,
    Vec(Vec<u8>),
    Ref(Bytes),
}

const HEADER_CAP: usize = 1024;

impl Header {
    fn as_slice(&self) -> &[u8] {
        match &self.buf {
            HeaderBuf::Vec(vec) => vec.as_slice(),
            HeaderBuf::Ref(bytes) => bytes.as_ref(),
            HeaderBuf::Empty => panic!("as_slice on empty header"),
        }
    }

    fn ready(&self) -> bool {
        let len = match &self.buf {
            HeaderBuf::Vec(vec) => vec.len(),
            HeaderBuf::Ref(bytes) => bytes.len(),
            HeaderBuf::Empty => 0,
        };

        len >= HEADER_CAP
    }

    fn add_chunk(&mut self, bytes: &Bytes) {
        if self.ready() {
            return;
        }

        self.buf = match std::mem::replace(&mut self.buf, HeaderBuf::Empty) {
            HeaderBuf::Empty => HeaderBuf::Ref(bytes.clone()),
            HeaderBuf::Vec(mut vec) => {
                let needed = HEADER_CAP - vec.len();
                let actual = needed.min(bytes.len());
                vec.extend(bytes.slice(0..actual));
                HeaderBuf::Vec(vec)
            }
            HeaderBuf::Ref(first_bytes) => {
                // We had a first chunk of bytes but it wasn't big enough, so
                // create a new contiguous buffer from the first chunk...
                let mut vec = Vec::with_capacity(HEADER_CAP);
                vec.extend(first_bytes.iter().take(HEADER_CAP));

                // And then add the current chunk
                let needed = HEADER_CAP - vec.len();
                let actual = needed.min(bytes.len());
                vec.extend(bytes.slice(0..actual));

                HeaderBuf::Vec(vec)
            }
        };
    }
}

async fn handle_upload(
    mut writer: impl AsyncWrite + Unpin,
    mut stream: BodyStream,
) -> Result<(String, ImageInfo), Error> {
    let mut hasher = blake3::Hasher::new();

    let mut header = Header::new();
    let mut info: Option<ImageInfo> = None;

    while let Some(chunk) = stream.try_next().await.transpose() {
        let chunk = chunk?;
        hasher.update(&chunk);

        if info.is_none() {
            header.add_chunk(&chunk);
            if header.ready() {
                let cursor = std::io::Cursor::new(header.as_slice());
                let mut reader = std::io::BufReader::new(cursor);
                let i = ImageInfo::from_reader(&mut reader)?;
                info = Some(i);
            }
        }

        writer.write_all(&chunk).await?;
    }

    let info = info.ok_or(Error::ImageHeaderDecode(ImageInfoError::UnrecognizedFormat))?;

    let hash = hasher.finalize();
    let hash_hex = format!("{:x?}", hash);

    Ok((hash_hex, info))
}

pub async fn upload_image(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    ContentLengthLimit(stream): ContentLengthLimit<BodyStream, { 250 * 1048576 }>,
    Path(image_id): Path<BaseImageId>,
) -> Result<impl IntoResponse, Error> {
    use db::{base_images, storage_locations, upload_profiles};

    let conn = state.db.get().await?;

    let (base_image, output_path, allowed) = conn
        .interact(move |conn| {
            base_images::table
                .inner_join(upload_profiles::table.inner_join(
                    storage_locations::table.on(storage_locations::storage_location_id
                        .eq(upload_profiles::base_storage_location_id)),
                ))
                .filter(base_images::base_image_id.eq(image_id))
                .filter(base_images::team_id.eq(user.team_id))
                .select((
                    base_images::all_columns,
                    storage_locations::all_columns,
                    db::obj_allowed!(
                        user.team_id,
                        &user.roles,
                        upload_profiles::project_id,
                        Permission::ImageCreate
                    ),
                ))
                .first::<(
                    base_images::BaseImage,
                    storage_locations::StorageLocation,
                    bool,
                )>(conn)
        })
        .await??;

    if !allowed {
        return Err(Error::MissingPermission(Permission::ImageCreate));
    }

    let provider = storage::Provider::from_db(output_path.provider)?;

    let operator = provider
        .create_operator(output_path.public_url_base.as_str())
        .await
        .map_err(storage::Error::OperatorError)?;

    let object = operator.object(base_image.location.as_str());
    let writer = object.writer(512 * 1024).await?;

    let (hash_hex, info) = handle_upload(writer, stream).await?;

    let format = match info.format {
        ImageFormat::PNG => db::ImageFormat::Png,
        ImageFormat::AVIF => db::ImageFormat::Avif,
        ImageFormat::JPEG => db::ImageFormat::Jpg,
        ImageFormat::WEBP => db::ImageFormat::Webp,
        _ => return Err(Error::ImageHeaderDecode(ImageInfoError::UnrecognizedFormat)),
    };

    let conn = state.db.get().await?;
    conn.interact(move |conn| {
        conn.transaction(|conn| {
            diesel::update(base_images::table)
                .filter(base_images::base_image_id.eq(image_id))
                .filter(base_images::team_id.eq(user.team_id))
                .set((
                    base_images::hash.eq(hash_hex),
                    base_images::format.eq(Some(format)),
                    base_images::width.eq(info.size.width as i32),
                    base_images::height.eq(info.size.height as i32),
                    base_images::status.eq(db::BaseImageStatus::Converting),
                ))
                .execute(conn)
        })

        // TODO Schedule conversions
    })
    .await??;

    Ok((StatusCode::OK, Json(json!({}))))
}
