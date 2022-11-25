use axum::{
    body::{Body, BoxBody},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::future::BoxFuture;
use serde_json::json;
use tower::{Layer, Service};

#[derive(Clone)]
pub struct ObfuscateErrorLayer {
    enabled: bool,
    obfuscate_forbidden: bool,
}

impl ObfuscateErrorLayer {
    pub fn new(enabled: bool, obfuscate_forbidden: bool) -> ObfuscateErrorLayer {
        ObfuscateErrorLayer {
            enabled,
            obfuscate_forbidden,
        }
    }
}

impl<S: Service<Request<Body>>> Layer<S> for ObfuscateErrorLayer {
    type Service = ObfuscateError<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ObfuscateError {
            inner,
            enabled: self.enabled,
            obfuscate_forbidden: self.obfuscate_forbidden,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObfuscateError<S> {
    inner: S,
    enabled: bool,
    obfuscate_forbidden: bool,
}

impl<S, B> Service<Request<B>> for ObfuscateError<S>
where
    S: Service<Request<B>> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: IntoResponse + Send + 'static,
    B: Send,
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

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let enabled = self.enabled;
        let obfuscate_forbidden = self.obfuscate_forbidden;
        let fut = self.inner.call(req);
        Box::pin(async move {
            let res = fut.await?.into_response();
            if !enabled {
                return Ok(res);
            }

            let status = res.status();
            let message = match status {
                StatusCode::INTERNAL_SERVER_ERROR => "Internal error",
                StatusCode::UNAUTHORIZED => {
                    if obfuscate_forbidden {
                        "Unauthorized"
                    } else {
                        ""
                    }
                }
                StatusCode::FORBIDDEN => "Forbidden",
                _ => "",
            };

            if message.is_empty() {
                // This is not an error we need to obfuscate
                return Ok(res);
            }

            let new_response = json!({
                "error": {
                    "detail": message,
                }
            });

            let new_res = (status, Json(new_response)).into_response();

            Ok(new_res)
        })
    }
}

#[cfg(test)]
mod test {
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    use super::ObfuscateErrorLayer;

    fn make_app(enabled: bool) -> Router {
        Router::new()
            .route("/200", get(|| async { (StatusCode::OK, "success") }))
            .route(
                "/500",
                get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "error 500") }),
            )
            .route(
                "/401",
                get(|| async { (StatusCode::UNAUTHORIZED, "error 401") }),
            )
            .route(
                "/403",
                get(|| async { (StatusCode::FORBIDDEN, "error 403") }),
            )
            .layer(ObfuscateErrorLayer::new(enabled, true))
    }

    async fn send_req(app: &Router, url: &str) -> (StatusCode, String) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(url)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn test_disabled() {
        let app = make_app(false);

        let (code, body) = send_req(&app, "/200").await;
        assert_eq!(code, 200, "/200 status code");
        assert_eq!(body, "success", "/200 body");

        let (code, body) = send_req(&app, "/401").await;
        assert_eq!(code, 401, "/401 status code");
        assert_eq!(body, "error 401", "/401 body");

        let (code, body) = send_req(&app, "/403").await;
        assert_eq!(code, 403, "/403 status code");
        assert_eq!(body, "error 403", "/403 body");

        let (code, body) = send_req(&app, "/500").await;
        assert_eq!(code, 500, "/500 status code");
        assert_eq!(body, "error 500", "/500 body");
    }

    #[tokio::test]
    async fn test_enabled() {
        let app = make_app(true);

        let (code, body) = send_req(&app, "/200").await;
        assert_eq!(code, 200, "/200 status code");
        assert_eq!(body, "success", "/200 body");

        let (code, body) = send_req(&app, "/401").await;
        assert_eq!(code, 401, "/401 status code");
        assert_eq!(
            body, r##"{"error":{"detail":"Unauthorized"}}"##,
            "/401 body should be obfuscated"
        );

        let (code, body) = send_req(&app, "/403").await;
        assert_eq!(code, 403, "/403 status code");
        assert_eq!(
            body, r##"{"error":{"detail":"Forbidden"}}"##,
            "/403 body should be obfuscated"
        );

        let (code, body) = send_req(&app, "/500").await;
        assert_eq!(code, 500, "/500 status code");
        assert_eq!(
            body, r##"{"error":{"detail":"Internal error"}}"##,
            "/500 body should be obfuscated"
        );
    }
}
