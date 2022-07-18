use diesel::prelude::*;
use diesel::sql_types;
use serde::{Deserialize, Serialize};

use crate::object_id::{ConversionProfileId, ProjectId, TeamId};
use crate::{diesel_jsonb, schema::*};

pub use crate::schema::conversion_profiles::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionSize {
    width: Option<usize>,
    height: Option<usize>,
    preserve_aspect_ratio: Option<bool>,
}

diesel_jsonb!(ConversionSize);

// This will eventually contain more details such as format-specific quality settings.
#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::Jsonb)]
#[serde(tag = "format")]
pub enum ConversionFormat {
    Png,
    Jpg,
    Avif,
    Webp,
}

diesel_jsonb!(ConversionFormat);

#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::Jsonb)]
pub struct ConversionOutput {
    formats: Vec<ConversionFormat>,
    sizes: Vec<ConversionSize>,
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

#[derive(Debug, Insertable)]
#[diesel(table_name = conversion_profiles)]
pub struct NewConversionProfile {
    pub conversion_profile_id: ConversionProfileId,
    pub team_id: TeamId,
    pub project_id: Option<ProjectId>,
    pub name: String,

    pub output: ConversionOutput,
}
