use diesel::prelude::*;

use crate::{
    object_id::{ProjectId, TeamId},
    schema::*,
};

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
