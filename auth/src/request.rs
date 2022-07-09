use std::marker::PhantomData;

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

pub struct AuthenticationLayer<APIKEYSTORE: ApiKeyStore, SESSIONSTORE: SessionStore, USERDATA>
where
    USERDATA: From<RequestUser<APIKEYSTORE::FetchData, SESSIONSTORE::SessionFetchData>>
        + Send
        + Sync
        + 'static,
{
    pub api_keys: ApiKeyManager<APIKEYSTORE>,
    pub sessions: SessionManager<SESSIONSTORE>,

    user_data_phantom: PhantomData<USERDATA>,
}

impl<APIKEYSTORE: ApiKeyStore, SESSIONSTORE: SessionStore, USERDATA>
    AuthenticationLayer<APIKEYSTORE, SESSIONSTORE, USERDATA>
where
    USERDATA: From<RequestUser<APIKEYSTORE::FetchData, SESSIONSTORE::SessionFetchData>>
        + Send
        + Sync
        + 'static,
{
    pub fn new(api_key_store: APIKEYSTORE, session_manager: SessionManager<SESSIONSTORE>) -> Self {
        Self {
            api_keys: ApiKeyManager {
                store: api_key_store,
            },
            sessions: session_manager,
            user_data_phantom: PhantomData,
        }
    }
}

impl<S: Service<Request<Body>>, APIKEYSTORE: ApiKeyStore, SESSIONSTORE: SessionStore, USERDATA>
    Layer<S> for AuthenticationLayer<APIKEYSTORE, SESSIONSTORE, USERDATA>
where
    USERDATA: From<RequestUser<APIKEYSTORE::FetchData, SESSIONSTORE::SessionFetchData>>
        + Send
        + Sync
        + 'static,
{
    type Service = Authenticator<S, APIKEYSTORE, SESSIONSTORE, USERDATA>;

    fn layer(&self, inner: S) -> Self::Service {
        Authenticator {
            api_keys: self.api_keys.clone(),
            sessions: self.sessions.clone(),
            user_data_phantom: PhantomData,
            inner,
        }
    }
}

pub enum RequestUser<ApiKeyData, SessionData> {
    ApiKey(ApiKeyData),
    Session(SessionData),
}

pub struct Authenticator<
    S: Service<Request<Body>>,
    APIKEYSTORE: ApiKeyStore,
    SESSIONSTORE: SessionStore,
    USERDATA,
> where
    USERDATA: From<RequestUser<APIKEYSTORE::FetchData, SESSIONSTORE::SessionFetchData>>,
{
    api_keys: ApiKeyManager<APIKEYSTORE>,
    sessions: SessionManager<SESSIONSTORE>,
    user_data_phantom: PhantomData<USERDATA>,
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

impl<S: Service<Request<Body>>, SESSIONSTORE: SessionStore, APIKEYSTORE: ApiKeyStore, USERDATA>
    Authenticator<S, APIKEYSTORE, SESSIONSTORE, USERDATA>
where
    USERDATA: From<RequestUser<APIKEYSTORE::FetchData, SESSIONSTORE::SessionFetchData>>,
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

impl<S, APIKEYSTORE, SESSIONSTORE, USERDATA> Service<Request<Body>>
    for Authenticator<S, APIKEYSTORE, SESSIONSTORE, USERDATA>
where
    S: Service<Request<Body>> + Send + Sync + Clone + 'static,
    S::Future: Send + 'static,
    S::Response: IntoResponse + Send + 'static,
    APIKEYSTORE: ApiKeyStore,
    SESSIONSTORE: SessionStore,
    USERDATA: From<RequestUser<APIKEYSTORE::FetchData, SESSIONSTORE::SessionFetchData>>
        + Send
        + Sync
        + 'static,
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
                    req.extensions_mut().insert(USERDATA::from(user));
                }
                Ok(None) => {}
                Err(e) => return Ok(e.into_response()),
            }

            Ok(inner.call(req).await?.into_response())
        })
    }
}
