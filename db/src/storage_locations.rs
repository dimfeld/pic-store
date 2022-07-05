use diesel::prelude::*;
use diesel::sql_types::Jsonb;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{diesel_jsonb, schema::*};

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
#[diesel(primary_key(storage_location_id))]
pub struct StorageLocation {
    pub storage_location_id: Uuid,
    pub team_id: Uuid,
    pub project_id: Option<Uuid>,
    pub name: String,

    pub provider: Provider,
    pub base_location: String,

    /// The base URL at which images in this StorageLocation can be accessed on the web.
    pub public_url_base: String,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = storage_locations)]
pub struct NewStorageLocation {
    pub storage_location_id: Uuid,
    pub team_id: Uuid,
    pub project_id: Option<Uuid>,
    pub name: String,

    pub provider: Provider,
    pub base_location: String,

    /// The base URL at which images in this StorageLocation can be accessed on the web.
    pub public_url_base: String,
}
