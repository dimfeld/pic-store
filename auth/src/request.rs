use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use axum::{
    body::{Body, Bytes, HttpBody},
    extract::{FromRequest, RequestParts},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    BoxError,
};
use biscuit_auth::Biscuit;
use futures::{future::BoxFuture, Future};
use thiserror::Error;
use tower::{Layer, Service};

use crate::{extract_token::invalid_message, AuthEvaluator, RootAuthEvaulator};

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

#[derive(Debug)]
pub struct CheckBiscuitLayer<T: BiscuitInfoExtractor> {
    root_auth: Arc<RootAuthEvaulator>,
    extractor: T,
}

impl<T: BiscuitInfoExtractor> CheckBiscuitLayer<T> {
    pub fn new(root_auth: Arc<RootAuthEvaulator>, extractor: T) -> Self {
        Self {
            root_auth,
            extractor,
        }
    }
}

impl<S, T: BiscuitInfoExtractor> Layer<S> for CheckBiscuitLayer<T> {
    type Service = CheckBiscuit<S, T>;

    fn layer(&self, inner: S) -> Self::Service {
        CheckBiscuit {
            inner,
            root_auth: self.root_auth.clone(),
            extractor: self.extractor.clone(),
        }
    }
}

#[derive(Debug)]
pub struct CheckBiscuit<S, T: BiscuitInfoExtractor> {
    inner: S,
    root_auth: Arc<RootAuthEvaulator>,
    extractor: T,
}

impl<S, T: BiscuitInfoExtractor> CheckBiscuit<S, T> {
    async fn check(
        root_auth: Arc<RootAuthEvaulator>,
        extractor: T,
        req: &Request<Body>,
    ) -> Result<T::OutputData, BiscuitExtractorError> {
        let token = req
            .extensions()
            .get::<BiscuitExtension>()
            .ok_or(BiscuitExtractorError::Unauthorized)?;

        let mut authorizer = root_auth.with_biscuit(token)?;

        let output = extractor.extract(req, &mut authorizer).await?;

        authorizer.authorize()?;

        Ok(output)
    }
}

impl<S, T: BiscuitInfoExtractor, ResBody> Service<Request<Body>> for CheckBiscuit<S, T>
where
    S: Service<Request<Body>, Response = Response<ResBody>> + Send + Sync + Clone + 'static,
    S::Future: Send + 'static,
    ResBody: HttpBody<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError> + Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let root_auth = self.root_auth.clone();
        let extractor = self.extractor.clone();

        Box::pin(async move {
            let check_future = Self::check(root_auth, extractor, &req);
            match check_future.await {
                Ok(output) => {
                    req.extensions_mut().insert::<T::OutputData>(output);
                }
                Err(e) => return Ok(e.into_response()),
            };

            Ok(inner.call(req).await?.map(axum::body::boxed))
        })
    }
}

#[derive(Debug, Error)]
pub enum BiscuitExtractorError {
    #[error("{}", crate::extract_token::INVALID_MESSAGE_BODY)]
    /// A generic unauthorized error
    Unauthorized,
    #[error("{}", crate::extract_token::INVALID_MESSAGE_BODY)]
    Token(#[from] biscuit_auth::error::Token),
    #[error(transparent)]
    InternalServerError(#[from] tower::BoxError),
}

impl From<crate::Error> for BiscuitExtractorError {
    fn from(err: crate::Error) -> Self {
        match err {
            crate::Error::TokenError(token) => BiscuitExtractorError::Token(token),
            _ => BiscuitExtractorError::Unauthorized,
        }
    }
}

impl IntoResponse for BiscuitExtractorError {
    fn into_response(self) -> Response {
        match self {
            Self::InternalServerError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
            _ => invalid_message(),
        }
    }
}

#[async_trait]
pub trait BiscuitInfoExtractor: Clone + Send + Sync + 'static {
    /// Non-aith data that the extractor fetches which might be useful to request handlers.
    /// For example, it might need to fetch the object being checked, and can set OutputData
    /// to that type.
    type OutputData: Send + Sync;

    async fn extract(
        &self,
        req: &Request<Body>,
        authorizer: &mut AuthEvaluator,
    ) -> Result<Self::OutputData, BiscuitExtractorError>;
}

#[async_trait]
impl<F, Fut, Output> BiscuitInfoExtractor for F
where
    F: Fn(&Request<Body>, &mut AuthEvaluator) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<Output, BiscuitExtractorError>> + Send,
    Output: Send + Sync + 'static,
{
    type OutputData = Output;

    async fn extract(
        &self,
        req: &Request<Body>,
        auth: &mut AuthEvaluator,
    ) -> Result<Output, BiscuitExtractorError> {
        (self)(req, auth).await
    }
}
