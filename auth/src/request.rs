use std::{fmt::Debug, ops::Deref, sync::Arc};

use async_trait::async_trait;
use axum::{
    body::{Body, Bytes, HttpBody},
    extract::{FromRequest, RequestParts},
    http::{header::HeaderName, HeaderValue, Request, StatusCode},
    response::{ErrorResponse, IntoResponse, Response},
    BoxError, Json,
};
use biscuit_auth::Biscuit;
use futures::{future::BoxFuture, Future, FutureExt, TryFutureExt};
use serde::Serialize;
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

// impl<F, Fut> CheckBiscuitLayer<ExtractorFnWrapper<F>>
// where
//     for<'r> F:
//         Fn(RequestParts<Body>, &mut AuthEvaluator<'r>) -> Fut + Clone + Send + Sync + 'static,
//     Fut: Future<Output = Result<RequestParts<Body>, BiscuitExtractorError>> + Send,
// {
//     pub fn from_fn(f: F) -> Self {
//         let extractor = ExtractorFnWrapper(f);
//         Self::new(extractor)
//     }
// }

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
    StringError(StatusCode, String),
    #[error("{0} {1}")]
    CustomError(StatusCode, String),
    #[error(transparent)]
    InternalServerError(#[from] tower::BoxError),
}

impl BiscuitExtractorError {
    pub fn internal_error(e: impl Into<tower::BoxError>) -> Self {
        Self::InternalServerError(e.into())
    }
}

impl<T> From<(StatusCode, T)> for BiscuitExtractorError
where
    T: Serialize + Debug,
{
    fn from(err: (StatusCode, T)) -> Self {
        BiscuitExtractorError::CustomError(
            err.0,
            serde_json::to_string(&err.1).unwrap_or_else(|_| format!("{:?}", err.1)),
        )
    }
}

impl From<crate::Error> for BiscuitExtractorError {
    fn from(err: crate::Error) -> Self {
        match err {
            crate::Error::TokenError(token) => BiscuitExtractorError::Token(token),
            crate::Error::MissingCredentials => {
                BiscuitExtractorError::StringError(StatusCode::UNAUTHORIZED, err.to_string())
            }
            _ => BiscuitExtractorError::StringError(
                StatusCode::INTERNAL_SERVER_ERROR,
                err.to_string(),
            ),
        }
    }
}

impl IntoResponse for BiscuitExtractorError {
    fn into_response(self) -> Response {
        match self {
            Self::InternalServerError(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
            Self::StringError(code, message) => (
                code,
                Json(pic_store_http_errors::ErrorResponseData::new(message)),
            )
                .into_response(),
            Self::CustomError(code, json) => {
                let mut response = (code, json).into_response();
                // The JSON is pre-serialized so we set the content-type manually.
                response
                    .headers_mut()
                    .insert("content-type", HeaderValue::from_static("application/json"));
                response
            }
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

// #[derive(Clone)]
// struct ExtractorFnWrapper<F>(F);

// #[async_trait]
// impl<F, Fut> BiscuitInfoExtractor for ExtractorFnWrapper<F>
// where
//     for<'a> F:
//         Fn(RequestParts<Body>, &'a mut AuthEvaluator<'a>) -> Fut + Clone + Send + Sync + 'static,
//     Fut: Future<Output = Result<RequestParts<Body>, BiscuitExtractorError>> + Send,
// {
//     async fn extract(
//         &self,
//         req: RequestParts<Body>,
//         authorizer: &'a mut AuthEvaluator<'a>,
//     ) -> Result<RequestParts<Body>, BiscuitExtractorError> {
//         (self.0)(req, authorizer).await
//     }
// }
