use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    object_id::{ProjectId, TeamId},
    schema::*,
};

pub use crate::schema::projects::*;

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(project_id))]
pub struct Project {
    pub project_id: ProjectId,
    pub team_id: TeamId,
    pub name: String,
    pub base_location: String,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Deserialize, Insertable)]
#[diesel(table_name = projects)]
pub struct NewProject {
    pub project_id: ProjectId,
    pub team_id: TeamId,
    pub name: String,
    pub base_location: String,

    pub updated: chrono::DateTime<chrono::Utc>,
}
