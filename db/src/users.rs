use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::*;

#[derive(Clone, Debug, Queryable, Identifiable)]
#[diesel(primary_key(user_id))]
pub struct User {
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub email: String,
    pub name: String,
    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub email: String,
    pub name: String,
}
