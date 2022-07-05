use diesel::prelude::*;
use uuid::Uuid;

use crate::enums::{BaseImageStatus, ImageFormat, OutputImageStatus};
use crate::schema::*;

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(base_image_id))]
pub struct BaseImage {
    pub base_image_id: Uuid,
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub project_id: Uuid,
    pub hash: String,

    /// The original filename of the image.
    pub filename: String,

    /// A subpath at which the image can be found, with the linked StorageLocation in the
    /// UploadProfile as the root location.
    pub location: String,

    pub width: i32,
    pub height: i32,
    pub format: ImageFormat,

    pub upload_profile_id: Uuid,
    pub status: BaseImageStatus,
    pub alt_text: String,
    pub placeholder: String,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = base_images)]
pub struct NewBaseImage {
    pub base_image_id: Uuid,
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub project_id: Uuid,
    pub hash: String,

    /// The original filename of the image.
    pub filename: String,

    /// A subpath at which the image can be found, with the linked StorageLocation in the
    /// UploadProfile as the root location.
    pub location: String,

    pub width: i32,
    pub height: i32,
    pub format: ImageFormat,

    pub upload_profile_id: Uuid,
    pub status: BaseImageStatus,
    pub alt_text: String,
    pub placeholder: String,
}

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(output_image_id))]
pub struct OutputImage {
    pub output_image_id: Uuid,
    pub team_id: Uuid,
    pub base_image_id: Uuid,
    pub location: String,
    pub width: i32,
    pub height: i32,
    pub format: ImageFormat,
    pub conversion_profile_item_id: Uuid,

    pub status: OutputImageStatus,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = output_images)]
pub struct NewOutputImage {
    pub output_image_id: Uuid,
    pub team_id: Uuid,
    pub base_image_id: Uuid,
    pub location: String,
    pub width: i32,
    pub height: i32,
    pub format: ImageFormat,
    pub conversion_profile_item_id: Uuid,

    pub status: OutputImageStatus,
}
