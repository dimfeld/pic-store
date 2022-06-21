use std::{fmt::Debug, ops::Deref, sync::Arc};

use async_trait::async_trait;
use axum::{
    body::{Body, Bytes, HttpBody},
    extract::{FromRequest, RequestParts},
    http::{Request, StatusCode},
    response::{ErrorResponse, IntoResponse, Response},
    BoxError, Json,
};
use biscuit_auth::Biscuit;
use futures::{future::BoxFuture, Future, FutureExt, TryFutureExt};
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
    extractor: T,
}

impl<T: BiscuitInfoExtractor> CheckBiscuitLayer<T> {
    pub fn new(extractor: T) -> Self {
        Self { extractor }
    }
}

impl<S, T: BiscuitInfoExtractor> Layer<S> for CheckBiscuitLayer<T> {
    type Service = CheckBiscuit<S, T>;

    fn layer(&self, inner: S) -> Self::Service {
        CheckBiscuit {
            inner,
            extractor: self.extractor.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CheckBiscuit<S, T: BiscuitInfoExtractor> {
    inner: S,
    extractor: T,
}

impl<S, T: BiscuitInfoExtractor> CheckBiscuit<S, T> {
    async fn check(
        root_auth: Arc<RootAuthEvaulator>,
        extractor: T,
        req: RequestParts<Body>,
    ) -> Result<RequestParts<Body>, BiscuitExtractorError> {
        let mut authorizer = root_auth.get_authorizer();

        let req = extractor.extract(req, &mut authorizer).await?;

        let token = req
            .extensions()
            .get::<BiscuitExtension>()
            .ok_or(BiscuitExtractorError::Unauthorized)?;

        let mut authorizer = authorizer.with_biscuit(token)?;
        authorizer.authorize()?;

        drop(authorizer);

        Ok(req)
    }
}

impl<S, T: BiscuitInfoExtractor, ResBody> Service<Request<Body>> for CheckBiscuit<S, T>
where
    S: Service<Request<Body>, Response = Response<ResBody>> + Send + Clone + 'static,
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

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let root_auth = req
            .extensions()
            .get::<Arc<RootAuthEvaulator>>()
            .cloned()
            .expect("RootAuthEvaulator is present on request");
        let extractor = self.extractor.clone();

        Box::pin(async move {
            let extract_result = Self::check(root_auth, extractor, RequestParts::new(req))
                .await
                .map_err(|e| e.into_response())
                .and_then(|req| req.try_into_request().map_err(|e| e.into_response()));

            match extract_result {
                Ok(req) => Ok(inner.call(req).await?.map(axum::body::boxed)),
                Err(e) => Ok(e),
            }
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
    #[error("{0} {1}")]
    CustomError(StatusCode, serde_json::Value),
    #[error(transparent)]
    InternalServerError(#[from] tower::BoxError),
}

impl BiscuitExtractorError {
    pub fn internal_error(e: impl Into<tower::BoxError>) -> Self {
        Self::InternalServerError(e.into())
    }
}

impl From<(StatusCode, serde_json::Value)> for BiscuitExtractorError {
    fn from(err: (StatusCode, serde_json::Value)) -> Self {
        BiscuitExtractorError::CustomError(err.0, err.1)
    }
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
            Self::InternalServerError(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
            Self::CustomError(code, json) => (code, Json(json)).into_response(),
            _ => invalid_message(),
        }
    }
}

#[async_trait]
pub trait BiscuitInfoExtractor: Clone + Send + Sync + 'static {
    async fn extract(
        &self,
        req: RequestParts<Body>,
        authorizer: &mut AuthEvaluator<'_>,
    ) -> Result<RequestParts<Body>, BiscuitExtractorError>;
}

// This is designed allow passing a function instead of an object to CheckBiscuitLayer,
// but I'm having trouble getting all the lifetimes and other requirements to line up.
// #[async_trait]
// impl<F, Fut> BiscuitInfoExtractor for F
// where
//     F: Fn(RequestParts<Body>, &mut AuthEvaluator<'static>) -> Fut + Clone + Send + Sync + 'static,
//     Fut: Future<Output = Result<RequestParts<Body>, BiscuitExtractorError>> + Send,
// {
//     async fn extract(
//         &self,
//         req: RequestParts<Body>,
//         auth: &mut AuthEvaluator<'static>,
//     ) -> Result<RequestParts<Body>, BiscuitExtractorError> {
//         (self)(req, auth).await
//     }
// }
