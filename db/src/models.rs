use diesel::prelude::*;
use diesel::sql_types::Jsonb;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{BaseImageStatus, ImageFormat, OutputImageStatus};
use crate::{diesel_jsonb, schema::*};

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(team_id))]
pub struct Team {
    pub team_id: Uuid,
    pub name: String,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(user_id))]
pub struct User {
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub email: String,
    pub name: String,
    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(project_id))]
pub struct Project {
    pub project_id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub base_location: String,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, AsExpression)]
#[serde(tag = "type")]
#[diesel(sql_type = Jsonb)]
pub enum Provider {
    /// Local filesystem
    Local,
    /// S3 or compatible storage
    S3 {
        endpoint: Option<String>,
        access_key_id: String,
        secret_key: String,
    },
}

diesel_jsonb!(Provider);

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(storage_location_id))]
pub struct StorageLocation {
    pub storage_location_id: Uuid,
    pub team_id: Uuid,
    pub name: String,

    pub provider: Provider,
    pub base_location: String,

    /// The base URL at which images in this StorageLocation can be accessed on the web.
    pub public_url_base: String,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(conversion_profile_id))]
pub struct ConversionProfile {
    pub conversion_profile_id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(conversion_profile_item_id))]
pub struct ConversionProfileItem {
    pub conversion_profile_item_id: Uuid,
    pub conversion_profile_id: Uuid,
    pub team_id: Uuid,
    pub name: String,

    pub format: ImageFormat,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(upload_profile_id))]
pub struct UploadProfile {
    pub upload_profile_id: Uuid,
    pub team_id: Uuid,
    pub project_id: Uuid,
    pub name: String,

    pub short_id: String,

    /// Where to store the input images, since they may not want to be in the same place as the
    /// output.
    pub base_storage_location_id: Uuid,
    /// Where to store the converted output images.
    pub output_storage_location_id: Uuid,
    pub conversion_profile_id: Uuid,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

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
