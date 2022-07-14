use diesel::prelude::*;

use crate::{
    object_id::{ConversionProfileId, ProjectId, StorageLocationId, TeamId, UploadProfileId},
    schema::*,
};

pub use crate::schema::upload_profiles::*;

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(upload_profile_id))]
pub struct UploadProfile {
    pub upload_profile_id: UploadProfileId,
    pub team_id: TeamId,
    pub project_id: ProjectId,
    pub name: String,

    pub short_id: Option<String>,

    /// Where to store the input images, since they may not want to be in the same place as the
    /// output.
    pub base_storage_location_id: StorageLocationId,
    /// Where to store the converted output images.
    pub output_storage_location_id: StorageLocationId,
    pub conversion_profile_id: ConversionProfileId,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = upload_profiles)]
pub struct NewUploadProfile {
    pub upload_profile_id: UploadProfileId,
    pub team_id: TeamId,
    pub project_id: ProjectId,
    pub name: String,

    pub short_id: String,

    /// Where to store the input images, since they may not want to be in the same place as the
    /// output.
    pub base_storage_location_id: StorageLocationId,
    /// Where to store the converted output images.
    pub output_storage_location_id: StorageLocationId,
    pub conversion_profile_id: ConversionProfileId,
}
