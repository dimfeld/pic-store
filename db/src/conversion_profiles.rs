use diesel::prelude::*;
use uuid::Uuid;

use crate::enums::ImageFormat;
use crate::schema::*;

pub use crate::schema::conversion_profiles::*;

#[derive(Clone, Debug, Queryable, Identifiable)]
#[diesel(primary_key(conversion_profile_id))]
pub struct ConversionProfile {
    pub conversion_profile_id: Uuid,
    pub team_id: Uuid,
    pub project_id: Option<Uuid>,
    pub name: String,
    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = conversion_profiles)]
pub struct NewConversionProfile {
    pub conversion_profile_id: Uuid,
    pub team_id: Uuid,
    pub project_id: Option<Uuid>,
    pub name: String,
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