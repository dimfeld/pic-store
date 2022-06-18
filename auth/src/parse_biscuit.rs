use axum::{
    body::{Body, Bytes, HttpBody},
    http::Request,
    response::Response,
    BoxError,
};
use biscuit_auth::{Biscuit, KeyPair, PrivateKey, PublicKey};
use futures::future::BoxFuture;
use tower::{Layer, Service};

use crate::extract_token::*;

pub struct BiscuitToken(String);

#[derive(Debug, Clone)]
pub enum TokenLocation {
    Bearer,
    Cookie(String),
}

#[derive(Clone)]
/// Extract a Biscuit token and verify it against the public key
pub struct BiscuitExtractorLayer {
    pubkey: PublicKey,
    token_location: TokenLocation,
}

impl BiscuitExtractorLayer {
    pub fn with_pubkey(pubkey: PublicKey) -> BiscuitExtractorLayer {
        BiscuitExtractorLayer {
            pubkey,
            token_location: TokenLocation::Bearer,
        }
    }

    pub fn with_token_location(self, token_location: TokenLocation) -> Self {
        BiscuitExtractorLayer {
            token_location,
            ..self
        }
    }
}

impl<S> Layer<S> for BiscuitExtractorLayer {
    type Service = BiscuitExtractor<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BiscuitExtractor {
            inner,
            pubkey: self.pubkey,
            token_location: self.token_location.clone(),
        }
    }
}

#[derive(Clone)]
/// Extract a Biscuit token and verify it against the public key
pub struct BiscuitExtractor<S> {
    inner: S,
    pubkey: PublicKey,
    token_location: TokenLocation,
}

impl<S, ResBody> Service<Request<Body>> for BiscuitExtractor<S>
where
    S: Service<Request<Body>, Response = Response<ResBody>> + Send + Clone + 'static,
    S::Future: Send + 'static,
    ResBody: HttpBody<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError>,
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
        let extracted = match &self.token_location {
            TokenLocation::Bearer => extract_bearer_auth_value(&req),
            TokenLocation::Cookie(cookie_name) => {
                Ok(extract_from_cookie(&req, cookie_name.as_str()))
            }
        };

        let parsed = extracted.transpose().map(|extracted| {
            extracted.and_then(|token| {
                Biscuit::from(token.as_ref(), |_| self.pubkey).map_err(|_| invalid_message())
            })
        });

        match parsed {
            None => {}
            Some(Ok(biscuit)) => {
                req.extensions_mut().insert(biscuit);
            }
            Some(Err(e)) => return Box::pin(async move { Ok(e) }),
        };

        let future = self.inner.call(req);
        Box::pin(async move {
            let response = future.await?.map(axum::body::boxed);
            Ok(response)
        })
    }
}

pub fn keypair_from_priv_key(key: PrivateKey) -> KeyPair {
    KeyPair::from(key)
}

pub fn keypair_from_priv_key_base64(encoded_key: &str) -> Result<KeyPair, anyhow::Error> {
    let bytes = base64::decode(encoded_key)?;
    let key = PrivateKey::from_bytes(&bytes)?;
    Ok(keypair_from_priv_key(key))
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
