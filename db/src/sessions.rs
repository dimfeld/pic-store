use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::{object_id::UserId, schema::*};

pub use crate::schema::sessions::*;

#[derive(Clone, Debug, Queryable, Identifiable, Insertable)]
pub struct Session {
    pub id: Uuid,
    pub user_id: UserId,
    pub expires: DateTime<Utc>,
}
