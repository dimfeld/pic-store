use diesel::prelude::*;

pub use crate::schema::base_images::*;
use crate::{
    enums::{BaseImageStatus, ImageFormat},
    object_id::{BaseImageId, ProjectId, TeamId, UploadProfileId, UserId},
    schema::*,
};

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
pub struct BaseImage {
    pub id: BaseImageId,
    pub team_id: TeamId,
    pub project_id: ProjectId,
    pub user_id: UserId,
    pub hash: Option<String>,

    /// The original filename of the image.
    pub filename: String,
    pub file_size: i32,

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

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = base_images)]
pub struct NewBaseImage {
    pub id: BaseImageId,
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
