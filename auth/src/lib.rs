use axum::{
    body::{Body, BoxBody},
    http::Request,
    response::Response,
};
use futures::future::BoxFuture;
use tower::{Layer, Service};

mod extract_header;
use extract_header::extract_bearer_auth_value;

pub struct BiscuitToken(String);

pub struct BiscuitVerifierLayer {
    pubkey: String,
}

impl BiscuitVerifierLayer {
    fn with_pubkey(pubkey: String) -> BiscuitVerifierLayer {
        BiscuitVerifierLayer { pubkey }
    }
}

impl<S> Layer<S> for BiscuitVerifierLayer {
    type Service = BiscuitVerifier<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BiscuitVerifier {
            inner,
            pubkey: self.pubkey.clone(),
        }
    }
}

#[derive(Clone)]
pub struct BiscuitVerifier<S> {
    inner: S,
    pubkey: String,
}

impl<S> Service<Request<Body>> for BiscuitVerifier<S>
where
    S: Service<Request<Body>, Response = Response> + Send + Clone + 'static,
    S::Future: Send + 'static,
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

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let inner_clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner_clone);

        match extract_bearer_auth_value(&req) {
            Ok(Some(token)) => {
                // TODO actually parse the biscuit
                req.extensions_mut().insert(BiscuitToken(token));
            }
            Ok(None) => {}
            Err(e) => return Box::pin(async move { Ok(e) }),
        };

        Box::pin(async move {
            let future = inner.call(req);
            let response = future.await?.map(axum::body::boxed);
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
