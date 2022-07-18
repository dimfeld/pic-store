use diesel::prelude::*;

use crate::enums::{BaseImageStatus, ImageFormat, OutputImageStatus};
use crate::object_id::{
    BaseImageId, ConversionProfileItemId, OutputImageId, ProjectId, TeamId, UploadProfileId, UserId,
};
use crate::schema::*;

pub use crate::schema::base_images::*;

#[derive(Clone, Debug, Queryable, Identifiable)]
#[diesel(primary_key(base_image_id))]
pub struct BaseImage {
    pub base_image_id: BaseImageId,
    pub team_id: TeamId,
    pub project_id: ProjectId,
    pub user_id: UserId,
    pub hash: Option<String>,

    /// The original filename of the image.
    pub filename: String,

    /// A subpath at which the image can be found, with the linked StorageLocation in the
    /// UploadProfile as the root location.
    pub location: String,

    pub width: i32,
    pub height: i32,
    pub format: Option<ImageFormat>,

    pub upload_profile_id: UploadProfileId,
    pub status: BaseImageStatus,
    pub alt_text: String,
    pub placeholder: Option<String>,

    pub uodated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = base_images)]
pub struct NewBaseImage {
    pub base_image_id: BaseImageId,
    pub user_id: UserId,
    pub team_id: TeamId,
    pub project_id: ProjectId,
    pub hash: String,

    /// The original filename of the image.
    pub filename: String,

    /// A subpath at which the image can be found, with the linked StorageLocation in the
    /// UploadProfile as the root location.
    pub location: String,

    pub width: i32,
    pub height: i32,
    pub format: Option<ImageFormat>,

    pub upload_profile_id: UploadProfileId,
    pub status: BaseImageStatus,
    pub alt_text: String,
    pub placeholder: String,
}
