use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::{object_id::UserId, schema::*};

pub use crate::schema::sessions::*;

#[derive(Clone, Debug, Queryable, Identifiable, Insertable)]
#[diesel(primary_key(session_id))]
pub struct Session {
    pub session_id: Uuid,
    pub user_id: UserId,
    pub expires: DateTime<Utc>,
}
