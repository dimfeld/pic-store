use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    object_id::{ProjectId, TeamId},
    schema::*,
};

pub use crate::schema::projects::*;

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
pub struct Project {
    pub id: ProjectId,
    pub team_id: TeamId,
    pub name: String,
    pub base_location: String,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Deserialize, Insertable)]
#[diesel(table_name = projects)]
pub struct NewProject {
    pub id: ProjectId,
    pub team_id: TeamId,
    pub name: String,
    pub base_location: String,
}
