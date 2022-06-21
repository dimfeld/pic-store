use std::future::Future;

use axum::{
    body::{boxed, Body, BoxBody},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use futures::future::{BoxFuture, FutureExt, MapOk};
use serde_json::json;
use tower::{util::MapResponseLayer, Layer, Service, ServiceExt};

pub struct ObfuscateErrorLayer {
    enabled: bool,
}

impl ObfuscateErrorLayer {
    pub fn new(enabled: bool) -> ObfuscateErrorLayer {
        ObfuscateErrorLayer { enabled }
    }
}

impl<S: Service<Request<Body>>> Layer<S> for ObfuscateErrorLayer {
    type Service = ObfuscateError<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ObfuscateError {
            inner,
            enabled: self.enabled,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObfuscateError<S> {
    inner: S,
    enabled: bool,
}

impl<S> Service<Request<Body>> for ObfuscateError<S>
where
    S: Service<Request<Body>> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: IntoResponse + Send + 'static,
{
    type Response = Response<BoxBody>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let enabled = self.enabled;
        let fut = self.inner.call(req);
        Box::pin(async move {
            let res = fut.await?.into_response();
            if !enabled {
                return Ok(res);
            }

            let status = res.status();
            let message = match status {
                StatusCode::INTERNAL_SERVER_ERROR => "Internal error",
                StatusCode::UNAUTHORIZED => "Unauthorized",
                StatusCode::FORBIDDEN => "Forbidden",
                _ => "",
            };

            if message.is_empty() {
                return Ok(res);
            }

            let new_response = json!({
                "error": {
                    "detail": message,
                }
            });

            let new_res = (status, message).into_response();

            Ok(new_res)
        })
    }
}
