use axum::{body::Body, http::Request};

use crate::{api_key::ApiKeyManager, session::SessionManager};

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

pub struct Authenticator<
    ApiKeyStore: crate::api_key::ApiKeyStore,
    SessionStore: crate::session::SessionStore,
> {
    pub api_keys: ApiKeyManager<ApiKeyStore>,
    pub sessions: SessionManager<SessionStore>,
}

// TODO Authenticator should be a middleware that first looks for an API key, and then a session cookie, and returns a
// Option<RequestUser> corresponding to what it finds.

impl<SessionStore: crate::session::SessionStore, ApiKeyStore: crate::api_key::ApiKeyStore>
    Authenticator<ApiKeyStore, SessionStore>
{
    async fn get_auth_info(
        &self,
        req: &Request<Body>,
    ) -> Result<
        Option<RequestUser<ApiKeyStore::FetchData, SessionStore::SessionFetchData>>,
        crate::Error,
    > {
        let key = self
            .api_keys
            .get_api_key(req)
            .await?
            .map(RequestUser::ApiKey);
        if key.is_some() {
            return Ok(key);
        }

        let session = self
            .sessions
            .get_session(req)
            .await?
            .map(RequestUser::Session);
        if session.is_some() {
            return Ok(session);
        }

        Ok(None)
    }
}
