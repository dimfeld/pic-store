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
    conversion_profiles::{self, ConversionOutput},
    object_id::{BaseImageId, OutputImageId, StorageLocationId},
    output_images::NewOutputImage,
    Permission, PoolExt,
};
use tracing::{event, Level};

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

    let (base_image, output_path, conversion_profile, allowed) = conn
        .interact(move |conn| {
            base_images::table
                .inner_join(
                    upload_profiles::table
                        .inner_join(
                            storage_locations::table.on(storage_locations::storage_location_id
                                .eq(upload_profiles::base_storage_location_id)),
                        )
                        .inner_join(conversion_profiles::table),
                )
                .filter(base_images::base_image_id.eq(image_id))
                .filter(base_images::team_id.eq(user.team_id))
                .select((
                    base_images::all_columns,
                    storage_locations::all_columns,
                    conversion_profiles::all_columns,
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
                    conversion_profiles::ConversionProfile,
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

    let basename = match base_image.location.rsplit_once('.') {
        Some((base, ext)) => base,
        None => base_image.location.as_str(),
    };

    let output_images = match &conversion_profile.output {
        ConversionOutput::Cross { formats, sizes } => formats
            .iter()
            .flat_map(|format| {
                sizes.iter().map(|size| {
                    let size_str = match (size.width, size.height) {
                        (Some(w), Some(h)) => format!("{w}x{h}"),
                        (Some(w), None) => format!("w{w}"),
                        (None, Some(h)) => format!("h{h}"),
                        (None, None) => {
                            event!(
                                Level::ERROR,
                                ?conversion_profile,
                                "Conversion profile has size-less item",
                            );
                            "szun".to_string()
                        }
                    };

                    let location = format!("{basename}-{size_str}.{}", format.extension());

                    NewOutputImage {
                        base_image_id: image_id,
                        output_image_id: OutputImageId::new(),
                        size: size.clone(),
                        format: format.clone(),
                        team_id: user.team_id,
                        status: db::OutputImageStatus::Queued,
                        location,
                    }
                })
            })
            .collect::<Vec<_>>(),
    };

    let output_image_ids = output_images
        .iter()
        .map(|oi| oi.output_image_id)
        .collect::<Vec<_>>();

    state
        .db
        .transaction(move |conn| {
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
                .execute(conn)?;

            diesel::insert_into(db::output_images::table)
                .values(&output_images)
                .execute(conn)?;

            Ok::<(), anyhow::Error>(())
        })
        .await?;

    let job_id = prefect::Job::builder(crate::jobs::CREATE_OUTPUT_IMAGES)
        .json_payload(&crate::jobs::CreateOutputImagesJobPayload {
            base_image: image_id,
            conversions: output_image_ids,
        })?
        .add_to(&state.queue)
        .await?;
    event!(Level::INFO, %job_id, "enqueued image conversion job");

    Ok((StatusCode::OK, Json(json!({}))))
}
