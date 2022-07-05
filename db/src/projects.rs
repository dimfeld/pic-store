use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::*;

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
