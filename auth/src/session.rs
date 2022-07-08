use std::borrow::Cow;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header::HOST, Request},
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use tower_cookies::Cookies;

#[async_trait]
pub trait SessionStore: Clone + Send + Sync + 'static {
    type UserId;
    type SessionFetchData: Send + Sync + 'static;
    type Error: IntoResponse + Send + Sync + 'static;

    async fn create_session(
        &self,
        id: Self::UserId,
        expires: DateTime<Utc>,
    ) -> Result<String, Self::Error>;
    async fn get_session(&self, id: &str) -> Result<Self::SessionFetchData, Self::Error>;
    async fn delete_session(&self, id: &str) -> Result<(), Self::Error>;
}

#[derive(Clone)]
pub struct SessionCookieManager {
    pub signing_key: tower_cookies::Key,
    pub cookie_name: String,
}

impl SessionCookieManager {
    pub fn set_session_cookie(
        &self,
        req: &Request<Body>,
        session_id: String,
        expire_days: i64,
        domain: Option<String>,
    ) {
        let cookie = tower_cookies::Cookie::build(self.cookie_name.clone(), session_id)
            .http_only(true)
            .secure(true)
            .max_age(time::Duration::days(expire_days));

        let cookie = if let Some(domain) = domain {
            cookie.domain(domain)
        } else {
            cookie
        };

        req.extensions()
            .get::<Cookies>()
            .unwrap()
            .signed(&self.signing_key)
            .add(cookie.finish());
    }

    pub fn get_session_cookie<'a, B>(&self, req: &'a Request<B>) -> Option<Cow<'a, str>> {
        let cookies = req.extensions().get::<Cookies>().unwrap();

        cookies
            .signed(&self.signing_key)
            .get(&self.cookie_name)
            .map(|cookie| {
                cookie
                    .value_raw()
                    .map(Cow::Borrowed)
                    .unwrap_or_else(|| Cow::Owned(cookie.value().to_string()))
            })
    }

    pub fn clear_session_cookie<B>(&self, req: &Request<B>) {
        let cookies = req.extensions().get::<Cookies>().unwrap();
        cookies.remove(tower_cookies::Cookie::new(self.cookie_name.clone(), ""))
    }
}

#[derive(Clone)]
pub struct SessionManager<Store: SessionStore> {
    pub cookies: SessionCookieManager,
    pub store: Store,
    pub expire_days: i64,
}

impl<Store: SessionStore> SessionManager<Store> {
    pub async fn create_session(
        &self,
        req: &Request<Body>,
        user_id: Store::UserId,
    ) -> Result<(), Store::Error> {
        let expiration = Utc::now() + chrono::Duration::days(self.expire_days);
        let session_id = self.store.create_session(user_id, expiration).await?;

        let domain = req
            .headers()
            .get(HOST)
            .and_then(|h| h.to_str().ok())
            .map(|h| h.to_string());

        self.cookies
            .set_session_cookie(req, session_id.to_string(), self.expire_days, domain);

        Ok(())
    }

    pub async fn get_session(
        &self,
        req: &Request<Body>,
    ) -> Result<Option<Store::SessionFetchData>, Store::Error> {
        let session_id = match self.cookies.get_session_cookie(req) {
            Some(session_id) => session_id,
            None => return Ok(None),
        };

        self.store.get_session(session_id.as_ref()).await.map(Some)
    }

    pub async fn delete_session(
        &self,
        req: &Request<Body>,
        session_id: &str,
    ) -> Result<(), Store::Error> {
        self.cookies.clear_session_cookie(req);
        self.store.delete_session(session_id).await
    }
}
