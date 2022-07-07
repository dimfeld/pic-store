use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::*;

#[derive(Clone, Debug, Queryable, Identifiable, Insertable)]
#[diesel(primary_key(session_id))]
pub struct Session {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub expires: DateTime<Utc>,
}
