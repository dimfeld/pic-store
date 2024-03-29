use axum::{
    extract::{BodyStream, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use bytes::Bytes;
use db::{
    base_images::BaseImage, conversion_profiles, image_base_location, object_id::BaseImageId,
    projects, Permission, PoolExt,
};
use diesel::prelude::*;
use futures::TryStreamExt;
use imageinfo::{ImageFormat, ImageInfo, ImageInfoError};
use pic_store_db as db;
use pic_store_storage as storage;
use serde_json::json;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tracing::{event, Level};

use crate::{
    auth::Authenticated,
    routes::image::{generate_output_images, replace_output_images},
    shared_state::AppState,
    Error,
};

struct Header {
    buf: HeaderBuf,
}

enum HeaderBuf {
    Empty,
    Vec(Vec<u8>),
    Ref(Bytes),
}

const HEADER_CAP: usize = 1024;

impl Header {
    fn new() -> Header {
        Header {
            buf: HeaderBuf::Empty,
        }
    }

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

    fn parse(&self) -> Result<ImageInfo, ImageInfoError> {
        assert!(self.ready());
        let bytes = self.as_slice();
        ImageInfo::from_raw_data(bytes)
    }
}

async fn handle_upload(
    upload: &mut Box<dyn AsyncWrite + Unpin + Send>,
    mut stream: BodyStream,
) -> Result<(String, usize, ImageInfo), Error> {
    let mut hasher = blake3::Hasher::new();

    let mut header = Header::new();
    let mut total_size = 0;
    let mut info: Option<ImageInfo> = None;

    while let Some(chunk) = stream.try_next().await? {
        hasher.update(&chunk);
        total_size += chunk.len();

        if info.is_none() {
            header.add_chunk(&chunk);
            if header.ready() {
                let i = header.parse()?;
                info = Some(i);
            }
        }

        upload.write_all(&chunk).await?;
    }

    let info = info.ok_or(Error::ImageHeaderDecode(ImageInfoError::UnrecognizedFormat))?;

    let hash = hasher.finalize();
    let hash_hex = hash.to_string();

    Ok((hash_hex, total_size, info))
}

pub async fn upload_image(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(image_id): Path<BaseImageId>,
    stream: BodyStream,
) -> Result<impl IntoResponse, Error> {
    use db::{base_images, storage_locations, upload_profiles};

    let conn = state.db.get().await?;

    let (
        base_image,
        output_path,
        conversion_profile,
        project_base_path,
        base_image_profile_location,
        allowed,
    ) = conn
        .interact(move |conn| {
            base_images::table
                .inner_join(
                    upload_profiles::table
                        .inner_join(storage_locations::table.on(
                            storage_locations::id.eq(upload_profiles::base_storage_location_id),
                        ))
                        .inner_join(conversion_profiles::table),
                )
                .inner_join(projects::table.on(projects::id.eq(base_images::project_id)))
                .filter(base_images::id.eq(image_id))
                .filter(base_images::team_id.eq(user.team_id))
                .select((
                    BaseImage::as_select(),
                    storage_locations::all_columns,
                    conversion_profiles::all_columns,
                    projects::base_location,
                    upload_profiles::base_storage_location_path,
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
                    String,
                    Option<String>,
                    bool,
                )>(conn)
        })
        .await??;

    if !allowed {
        return Err(Error::MissingPermission(Permission::ImageCreate));
    }

    let provider = storage::Provider::from_db(output_path.provider)?;

    let output_base_location = image_base_location(
        &output_path.base_location,
        &project_base_path,
        &base_image_profile_location,
    );

    let operator = provider
        .create_operator(output_base_location.as_ref())
        .await?;

    let (upload_id, mut writer) = operator.put_multipart(&base_image.location).await?;
    let (hash_hex, total_size, info) = match handle_upload(&mut writer, stream).await {
        Ok(result) => {
            writer.shutdown().await?;
            result
        }
        Err(e) => {
            operator
                .abort_multipart(&base_image.location, &upload_id)
                .await
                .ok();
            return Err(e);
        }
    };

    let upload_format = match info.format {
        ImageFormat::PNG => db::ImageFormat::Png,
        ImageFormat::AVIF => db::ImageFormat::Avif,
        ImageFormat::JPEG => db::ImageFormat::Jpg,
        ImageFormat::WEBP => db::ImageFormat::Webp,
        _ => return Err(Error::ImageHeaderDecode(ImageInfoError::UnrecognizedFormat)),
    };

    let output_images = generate_output_images(
        user.team_id,
        &conversion_profile,
        base_image.id,
        &base_image.location,
        upload_format,
    );

    let output_image_ids = state
        .db
        .transaction(move |conn| {
            diesel::update(base_images::table)
                .filter(base_images::id.eq(image_id))
                .filter(base_images::team_id.eq(user.team_id))
                .set((
                    base_images::hash.eq(hash_hex),
                    base_images::file_size.eq(total_size as i32),
                    base_images::format.eq(Some(upload_format)),
                    base_images::width.eq(info.size.width as i32),
                    base_images::height.eq(info.size.height as i32),
                    base_images::status.eq(db::BaseImageStatus::Converting),
                ))
                .execute(conn)?;
            replace_output_images(conn, user.team_id, image_id, output_images)
        })
        .await?;

    let job_id = effectum::Job::builder(crate::jobs::CREATE_OUTPUT_IMAGES)
        .json_payload(&crate::jobs::CreateOutputImagesJobPayload {
            base_image: image_id,
            conversions: output_image_ids,
        })?
        .add_to(&state.queue)
        .await?;
    event!(Level::INFO, %job_id, "enqueued image conversion job");

    Ok((StatusCode::OK, Json(json!({}))))
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read, path::PathBuf};

    use bytes::Bytes;
    use imageinfo::*;

    fn read_test_image_header(filename: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures")
            .join(filename);
        let mut file = File::open(path).expect("opening input file");
        let buf_len = std::cmp::min(
            file.metadata().unwrap().len(),
            (super::HEADER_CAP + 10) as u64,
        );

        let mut buf = vec![0; buf_len as usize];
        file.read_exact(&mut buf).expect("reading input file");
        buf
    }

    #[test]
    fn header_avif() {
        let file = read_test_image_header("test-input.avif");
        let mut header = super::Header::new();
        header.add_chunk(&Bytes::from(file));

        assert!(header.ready());
        let info = header.parse().unwrap();

        assert_eq!(info.format, ImageFormat::AVIF);
        assert_eq!(info.size.width, 1334);
        assert_eq!(info.size.height, 890);
    }

    #[test]
    fn header_jpg() {
        let file = read_test_image_header("test-input.jpeg");
        let mut header = super::Header::new();
        header.add_chunk(&Bytes::from(file));

        assert!(header.ready());
        let info = header.parse().unwrap();

        assert_eq!(info.format, ImageFormat::JPEG);
        assert_eq!(info.size.width, 1334);
        assert_eq!(info.size.height, 890);
    }

    #[test]
    fn header_png() {
        let file = read_test_image_header("test-input.png");
        let mut header = super::Header::new();
        header.add_chunk(&Bytes::from(file));

        assert!(header.ready());
        let info = header.parse().unwrap();

        assert_eq!(info.format, ImageFormat::PNG);
        assert_eq!(info.size.width, 667);
        assert_eq!(info.size.height, 445);
    }

    #[test]
    fn header_webp() {
        let file = read_test_image_header("test-input.webp");
        let mut header = super::Header::new();
        header.add_chunk(&Bytes::from(file));

        assert!(header.ready());
        let info = header.parse().unwrap();

        assert_eq!(info.format, ImageFormat::WEBP);
        assert_eq!(info.size.width, 1334);
        assert_eq!(info.size.height, 890);
    }

    /// Ensure that the Header class properly handles multiple small chunks
    #[test]
    fn header_small_chunks() {
        // AVIF has the most complex header, so use that.
        let file = read_test_image_header("test-input.avif");
        let mut header = super::Header::new();

        for chunk in file.chunks(100) {
            let chunk = Vec::from(chunk);
            header.add_chunk(&Bytes::from(chunk));
        }

        assert!(header.ready());
        let info = header.parse().unwrap();
        assert_eq!(info.format, ImageFormat::AVIF);
        assert_eq!(info.size.width, 1334);
        assert_eq!(info.size.height, 890);
    }
}
