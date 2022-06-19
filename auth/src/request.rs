use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, RequestParts},
    http::StatusCode,
};
use biscuit_auth::Biscuit;

use crate::AuthEvaluator;

pub type BiscuitExtension = Arc<Biscuit>;

#[derive(Debug)]
pub struct RequireBiscuit(pub BiscuitExtension);

#[async_trait]
impl<B> FromRequest<B> for RequireBiscuit
where
    B: Send,
{
    type Rejection = StatusCode;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let biscuit = req
            .extensions()
            .get::<BiscuitExtension>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)?;

        Ok(Self(biscuit))
    }
}

impl Deref for RequireBiscuit {
    type Target = Biscuit;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
