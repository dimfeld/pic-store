use diesel::prelude::*;
use diesel::sql_types::Jsonb;
use serde::{Deserialize, Serialize};

use crate::{
    diesel_jsonb,
    object_id::{ProjectId, StorageLocationId, TeamId},
    schema::*,
};

pub use crate::schema::storage_locations::*;

#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Provider {
    /// Local filesystem
    Local,
    /// S3 or compatible storage
    S3 {
        endpoint: Option<String>,
        region: Option<String>,
        access_key_id: Option<String>,
        secret_key: Option<String>,
        virtual_host_style: Option<bool>,
    },
}

diesel_jsonb!(Provider);

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match self {
            Self::Local => "local",
            Self::S3 { .. } => "s3",
        };

        f.write_str(desc)
    }
}

#[derive(Clone, Debug, Queryable, Identifiable)]
pub struct StorageLocation {
    pub id: StorageLocationId,
    pub team_id: TeamId,
    pub project_id: Option<ProjectId>,
    pub name: String,

    pub provider: Provider,
    pub base_location: String,

    /// The base URL at which images in this StorageLocation can be accessed on the web.
    pub public_url_base: String,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = storage_locations)]
pub struct NewStorageLocation {
    pub id: StorageLocationId,
    pub team_id: TeamId,
    pub project_id: Option<ProjectId>,
    pub name: String,

    pub provider: Provider,
    pub base_location: String,

    /// The base URL at which images in this StorageLocation can be accessed on the web.
    pub public_url_base: String,
}
