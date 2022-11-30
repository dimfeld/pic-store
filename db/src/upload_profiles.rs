use diesel::prelude::*;
use serde::Deserialize;

pub use crate::schema::upload_profiles::*;
use crate::{
    object_id::{ConversionProfileId, ProjectId, StorageLocationId, TeamId, UploadProfileId},
    schema::*,
};

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
pub struct UploadProfile {
    pub id: UploadProfileId,
    pub team_id: TeamId,
    pub project_id: ProjectId,
    pub name: String,

    pub short_id: Option<String>,

    /// Where to store the input images, since they may not want to be in the same place as the
    /// output.
    pub base_storage_location_id: StorageLocationId,
    /// A path within the base storage location where the base image will be stored.
    pub base_storage_location_path: String,
    /// Where to store the converted output images.
    pub output_storage_location_id: StorageLocationId,
    /// A path within the output storage location where the output images will be stored.
    pub output_storage_location_path: String,
    pub conversion_profile_id: ConversionProfileId,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = upload_profiles)]
pub struct NewUploadProfile {
    pub id: UploadProfileId,
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
