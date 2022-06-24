use axum::{
    extract::{BodyStream, ContentLengthLimit, Multipart, Path},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use blake2::Digest;
use bytes::Bytes;
use futures::{AsyncWriteExt, TryStreamExt};
use imageinfo::{ImageFormat, ImageInfo, ImageInfoError};
use sea_orm::{EntityTrait, Set};
use serde::Serialize;
use serde_json::json;
use time::OffsetDateTime;
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
        match self.buf {
            HeaderBuf::Vec(vec) => vec.as_slice(),
            HeaderBuf::Ref(bytes) => bytes.as_ref(),
            HeaderBuf::Empty => panic!("as_slice on empty header"),
        }
    }

    fn ready(&self) -> bool {
        let len = match self.buf {
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

        self.buf = match self.buf {
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

async fn upload_image(
    Extension(ref state): Extension<State>,
    Path((profile_id, file_name)): Path<(Uuid, String)>,
    ContentLengthLimit(mut stream): ContentLengthLimit<BodyStream, { 250 * 1048576 }>,
) -> Result<impl IntoResponse, Error> {
    // TODO once it's built out this will fetch from the database
    let team_id = state.team_id;
    let user_id = state.user_id;
    let output_path = db::storage_location::Model {
        id: Uuid::new_v4(),
        project_id: state.project_id,
        name: "test storage location".to_string(),
        provider: db::storage_location::Provider::Local,
        base_location: "./test_uploads".to_string(),
        credentials: None,
        public_url_base: "https://images.example.com".to_string(),
        updated: OffsetDateTime::now_utc(),
        deleted: None,
    };

    let provider = storage::Provider::from_db(
        output_path.provider,
        output_path
            .credentials
            .as_ref()
            .unwrap_or(&serde_json::Value::Null),
    )?;

    let operator = provider
        .create_operator(output_path.public_url_base.as_str())
        .await
        .map_err(storage::Error::OperatorError)?;

    let object = operator.object(file_name.as_str());
    let mut writer = object.writer(512 * 1024).await?;

    let mut hasher = blake2::Blake2b512::new();

    let mut header = Header::new();
    let mut info: Option<ImageInfo>;

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

    let format = match info.format {
        ImageFormat::PNG => "png",
        ImageFormat::AVIF => "avif",
        ImageFormat::JPEG => "jpeg",
        ImageFormat::GIF => "gif",
        ImageFormat::WEBP => "webp",
        _ => return Err(Error::ImageHeaderDecode(ImageInfoError::UnrecognizedFormat)),
    };

    let image_id = Uuid::new_v4();
    let base_image = db::base_image::ActiveModel {
        id: Set(image_id),
        project_id: Set(output_path.project_id),
        hash: Set(hash_hex),
        format: Set(format.to_string()),
        location: Set(file_name.clone()),
        filename: Set(file_name),
        width: Set(info.size.width as u32),
        height: Set(info.size.height as u32),
        upload_profile_id: Set(profile_id),
        user_id: Set(state.user_id),
        team_id: Set(state.team_id),
        updated: Set(OffsetDateTime::now_utc()),
        ..Default::default()
    };

    db::base_image::Entity::insert(base_image)
        .exec(&state.db)
        .await?;

    // TODO Schedule conversions

    Ok((StatusCode::OK, Json(json!({}))))
}

#[derive(Serialize)]
pub struct GetUploadUrlResponse {
    id: Uuid,
    url: Option<String>,
}

async fn get_upload_url(
    Path((profile_id, file_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    // TODO once it's built out this will fetch from the database
    let output_path = db::storage_location::Model {
        id: Uuid::new_v4(),
        project_id: Uuid::new_v4(),
        name: "test storage location".to_string(),
        provider: db::storage_location::Provider::Local,
        base_location: "./test_uploads".to_string(),
        credentials: None,
        public_url_base: "https://images.example.com".to_string(),
        updated: OffsetDateTime::now_utc(),
        deleted: None,
    };

    let provider = storage::Provider::from_db(
        output_path.provider,
        output_path
            .credentials
            .as_ref()
            .unwrap_or(&serde_json::Value::Null),
    )?;

    let destination = format!("{}/{}", output_path.base_location, file_name);

    let presigned_url = provider
        .create_presigned_upload_url(destination.as_str())
        .await?;

    // Add the entry to the database with some sort of pending tag
    // The client then uploads it to the backing store, and calls another endpoint (TBD) to mark it
    // done.

    let headers = presigned_url
        .headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or_default()))
        .filter(|(_, v)| !v.is_empty())
        .collect::<Vec<_>>();

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "method": presigned_url.method.as_str(),
            "uri": presigned_url.uri.to_string(),
            "headers": headers,
        })),
    ))
}

async fn list_profiles() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn write_profile() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn new_profile() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn get_profile(Path(profile_id): Path<Uuid>) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, profile_id.to_string())
}

async fn disable_profile() -> impl IntoResponse {
    todo!();
    StatusCode::NOT_IMPLEMENTED
}

pub fn configure() -> Router {
    Router::new()
        .route("/", get(list_profiles))
        .route("/", post(new_profile))
        .route("/:profile_id", get(get_profile))
        .route("/:profile_id", put(write_profile))
        .route("/:profile_id", delete(disable_profile))
        .route(
            "/:profile_id/get_upload_url/:file_name",
            post(get_upload_url),
        )
        .route("/:profile_id/upload/:file_name", post(upload_image))
}
