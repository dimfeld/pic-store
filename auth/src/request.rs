use axum::{
    body::{Body, BoxBody},
    http::{Request, Response},
    response::IntoResponse,
};
use futures::future::BoxFuture;
use tower::{Layer, Service};

use crate::{
    api_key::{ApiKeyManager, ApiKeyStore},
    session::{SessionManager, SessionStore},
};

// API Key
// Session
//
// Session auth requires
//  Login - validate password
//  Cookie auth - Check the session
//

pub enum RequestUser<ApiKeyData, SessionData> {
    ApiKey(ApiKeyData),
    Session(SessionData),
}

pub struct AuthenticationLayer<APIKEYSTORE: ApiKeyStore, SESSIONSTORE: SessionStore> {
    pub api_keys: ApiKeyManager<APIKEYSTORE>,
    pub sessions: SessionManager<SESSIONSTORE>,
}

impl<APIKEYSTORE: ApiKeyStore, SESSIONSTORE: SessionStore>
    AuthenticationLayer<APIKEYSTORE, SESSIONSTORE>
{
    pub fn new(api_key_store: APIKEYSTORE, session_manager: SessionManager<SESSIONSTORE>) -> Self {
        Self {
            api_keys: ApiKeyManager {
                store: api_key_store,
            },
            sessions: session_manager,
        }
    }
}

impl<S: Service<Request<Body>>, APIKEYSTORE: ApiKeyStore, SESSIONSTORE: SessionStore> Layer<S>
    for AuthenticationLayer<APIKEYSTORE, SESSIONSTORE>
{
    type Service = Authenticator<S, APIKEYSTORE, SESSIONSTORE>;

    fn layer(&self, inner: S) -> Self::Service {
        Authenticator {
            api_keys: self.api_keys.clone(),
            sessions: self.sessions.clone(),
            inner,
        }
    }
}

pub struct Authenticator<
    S: Service<Request<Body>>,
    APIKEYSTORE: ApiKeyStore,
    SESSIONSTORE: SessionStore,
> {
    api_keys: ApiKeyManager<APIKEYSTORE>,
    sessions: SessionManager<SESSIONSTORE>,
    inner: S,
}

pub enum AuthenticatorError<APIKEYSTORE: ApiKeyStore, SESSIONSTORE: SessionStore> {
    ApiKeyStore(APIKEYSTORE::Error),
    SessionStore(SESSIONSTORE::Error),
}

impl<APIKEYSTORE: ApiKeyStore, SESSIONSTORE: SessionStore> IntoResponse
    for AuthenticatorError<APIKEYSTORE, SESSIONSTORE>
{
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::ApiKeyStore(e) => e.into_response(),
            Self::SessionStore(e) => e.into_response(),
        }
    }
}

impl<S: Service<Request<Body>>, SESSIONSTORE: SessionStore, APIKEYSTORE: ApiKeyStore>
    Authenticator<S, APIKEYSTORE, SESSIONSTORE>
{
    async fn get_auth_info(
        api_keys: ApiKeyManager<APIKEYSTORE>,
        sessions: SessionManager<SESSIONSTORE>,
        req: &Request<Body>,
    ) -> Result<
        Option<RequestUser<APIKEYSTORE::FetchData, SESSIONSTORE::SessionFetchData>>,
        AuthenticatorError<APIKEYSTORE, SESSIONSTORE>,
    > {
        let key = api_keys
            .get_api_key(req)
            .await
            .map_err(AuthenticatorError::ApiKeyStore)?
            .map(RequestUser::ApiKey);
        if key.is_some() {
            return Ok(key);
        }

        let session = sessions
            .get_session(req)
            .await
            .map_err(AuthenticatorError::SessionStore)?
            .map(RequestUser::Session);
        if session.is_some() {
            return Ok(session);
        }

        Ok(None)
    }
}

impl<S, APIKEYSTORE, SESSIONSTORE> Service<Request<Body>>
    for Authenticator<S, APIKEYSTORE, SESSIONSTORE>
where
    S: Service<Request<Body>> + Send + Sync + Clone + 'static,
    S::Future: Send + 'static,
    S::Response: IntoResponse + Send + 'static,
    APIKEYSTORE: ApiKeyStore,
    SESSIONSTORE: SessionStore,
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
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);

        let api_keys = self.api_keys.clone();
        let sessions = self.sessions.clone();
        Box::pin(async move {
            let auth_result = Self::get_auth_info(api_keys, sessions, &req).await;
            match auth_result {
                Ok(Some(user)) => {
                    req.extensions_mut().insert(user);
                }
                Ok(None) => {}
                Err(e) => return Ok(e.into_response()),
            }

            Ok(inner.call(req).await?.into_response())
        })
    }
}
