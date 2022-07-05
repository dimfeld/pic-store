use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::*;

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

#[derive(Debug, Insertable)]
#[diesel(table_name = upload_profiles)]
pub struct NewUploadProfile {
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
}
