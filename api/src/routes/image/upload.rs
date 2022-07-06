use axum::{
    extract::{BodyStream, ContentLengthLimit, Multipart, Path},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use blake2::Digest;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use futures::{AsyncWrite, AsyncWriteExt, TryStreamExt};
use imageinfo::{ImageFormat, ImageInfo, ImageInfoError};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use pic_store_db as db;
use pic_store_storage as storage;

use crate::{shared_state::State, Error};

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

        return len >= HEADER_CAP;
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
    let mut hasher = blake2::Blake2b512::new();

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
    Path(image_id): Path<Uuid>,
    ContentLengthLimit(mut stream): ContentLengthLimit<BodyStream, { 250 * 1048576 }>,
) -> Result<impl IntoResponse, Error> {
    // TODO once it's built out this will fetch from the database
    let team_id = state.team_id;
    let user_id = state.user_id;
    let output_path = db::storage_locations::StorageLocation {
        storage_location_id: Uuid::new_v4(),
        team_id: state.team_id,
        project_id: None,
        name: "test storage location".to_string(),
        provider: db::storage_locations::Provider::Local,
        base_location: "./test_uploads".to_string(),
        public_url_base: "https://images.example.com".to_string(),
        updated: Utc::now(),
        deleted: None,
    };

    let provider = storage::Provider::from_db(output_path.provider)?;

    let operator = provider
        .create_operator(output_path.public_url_base.as_str())
        .await
        .map_err(storage::Error::OperatorError)?;

    let file_name = "FAKE_FILENAME".to_string(); // TODO
    let object = operator.object(file_name.as_str());
    let mut writer = object.writer(512 * 1024).await?;

    let (hash_hex, info) = handle_upload(writer, stream).await?;

    let format = match info.format {
        ImageFormat::PNG => db::ImageFormat::Png,
        ImageFormat::AVIF => db::ImageFormat::Avif,
        ImageFormat::JPEG => db::ImageFormat::Jpg,
        ImageFormat::WEBP => db::ImageFormat::Webp,
        _ => return Err(Error::ImageHeaderDecode(ImageInfoError::UnrecognizedFormat)),
    };

    let image_id = Uuid::new_v4();
    let base_image = db::images::NewBaseImage {
        base_image_id: Uuid::new_v4(),
        project_id: Uuid::new_v4(),
        hash: hash_hex,
        format: Some(format),
        location: file_name.clone(),
        filename: file_name,
        width: info.size.width as i32,
        height: info.size.height as i32,
        upload_profile_id: Uuid::new_v4(),
        user_id: state.user_id,
        team_id: state.team_id,
        alt_text: String::new(),
        placeholder: String::new(),
        // We already started the upload
        status: db::BaseImageStatus::Converting,
    };

    let conn = state.db.get().await?;
    let result = conn
        .interact(move |conn| {
            diesel::insert_into(db::base_images::table)
                .values(&base_image)
                .returning(db::base_images::all_columns)
                .get_result::<db::images::BaseImage>(conn)
        })
        .await?;

    // TODO Schedule conversions

    Ok((StatusCode::OK, Json(json!({}))))
}
