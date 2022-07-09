use diesel::prelude::*;

use crate::{object_id::TeamId, schema::*};

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(team_id))]
pub struct Team {
    pub team_id: TeamId,
    pub name: String,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = teams)]
pub struct NewTeam {
    pub team_id: TeamId,
    pub name: String,
}
