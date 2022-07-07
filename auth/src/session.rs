use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[async_trait]
pub trait SessionStore {
    type SessionId: ToString;
    type UserId;
    type Error;

    async fn add_session(
        id: Self::UserId,
        expires: DateTime<Utc>,
    ) -> Result<Self::SessionId, Self::Error>;
    async fn lookup_session(id: Self::SessionId) -> Result<Self::UserId, Self::Error>;
    async fn delete_session(id: Self::SessionId) -> Result<(), Self::Error>;
}
