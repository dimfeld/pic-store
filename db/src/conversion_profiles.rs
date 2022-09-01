use diesel::prelude::*;
use diesel::sql_types;
use serde::{Deserialize, Serialize};

use crate::object_id::{ConversionProfileId, ProjectId, TeamId};
use crate::{diesel_jsonb, schema::*};

pub use crate::schema::conversion_profiles::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::Jsonb)]
pub struct ConversionSize {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub preserve_aspect_ratio: Option<bool>,
}

diesel_jsonb!(ConversionSize);

// This will eventually contain more details such as format-specific quality settings.
#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::Jsonb)]
#[serde(tag = "format", rename_all = "lowercase")]
pub enum ConversionFormat {
    Png,
    Jpg,
    Avif,
    Webp,
}

diesel_jsonb!(ConversionFormat);

impl ConversionFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Avif => "avif",
            Self::Webp => "webp",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::Jsonb)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConversionOutput {
    Cross {
        formats: Vec<ConversionFormat>,
        sizes: Vec<ConversionSize>,
    },
}

diesel_jsonb!(ConversionOutput);

#[derive(Clone, Debug, Queryable, Identifiable)]
#[diesel(primary_key(conversion_profile_id))]
pub struct ConversionProfile {
    pub conversion_profile_id: ConversionProfileId,
    pub team_id: TeamId,
    pub project_id: Option<ProjectId>,
    pub name: String,

    pub output: ConversionOutput,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = conversion_profiles)]
pub struct NewConversionProfile {
    pub conversion_profile_id: ConversionProfileId,
    pub team_id: TeamId,
    pub project_id: Option<ProjectId>,
    pub name: String,

    pub output: ConversionOutput,
}
