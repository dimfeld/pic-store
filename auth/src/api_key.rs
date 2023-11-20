use std::borrow::Borrow;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request},
    response::IntoResponse,
};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::error::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiKey {
    pub api_key_id: Uuid,
    pub prefix: String,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>,
    pub inherits_user_permissions: bool,
    pub description: Option<String>,
    pub active: bool,
    pub expires: Option<DateTime<Utc>>,
    pub created: DateTime<Utc>,
}

pub type Hash = blake3::Hash;

#[derive(Debug, Clone)]
pub struct ApiKeyData {
    pub id: Uuid,
    pub key: String,
    pub prefix: String,
    pub hash: Hash,
    pub expires: DateTime<Utc>,
}

impl ApiKeyData {
    pub fn new<STORE: ApiKeyStore>(store: &STORE, expires: DateTime<Utc>) -> ApiKeyData {
        let id = Uuid::new_v4();
        let random = Uuid::new_v4();

        Self::from_params(store.api_key_prefix(), id, random, expires)
    }

    /// Create an API key with pre-filled ID and random data. This should usually only be used
    /// for bootstrapping purposes when you want to create a key deterministically.
    pub fn from_params(prefix: &str, id: Uuid, random: Uuid, expires: DateTime<Utc>) -> ApiKeyData {
        let engine = &base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let base64_id = engine.encode(id.as_bytes());
        let random = engine.encode(random.as_bytes());
        let key = format!("{prefix}.{base64_id}.{random}");
        let prefix = key[0..16].to_string();
        let hash = hash_key(&key);

        ApiKeyData {
            id,
            key,
            prefix,
            hash,
            expires,
        }
    }
}

fn hash_key(key: &str) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(key.as_bytes());
    hasher.finalize()
}

fn decode_key<STORE: ApiKeyStore>(store: &STORE, key: &str) -> Result<(Uuid, Hash), Error> {
    let prefix = store.api_key_prefix();
    if key.len() != 46 + prefix.len() || !key.starts_with(prefix) {
        return Err(Error::InvalidApiKeyFormat);
    }

    let hash = hash_key(key);
    let id_portion = key.split('.').nth(1).ok_or(Error::InvalidApiKeyFormat)?;
    let engine = &base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let api_key_bytes = engine
        .decode(id_portion.as_bytes())
        .map_err(|_| Error::InvalidApiKeyFormat)?;
    let api_key_id = Uuid::from_slice(&api_key_bytes).map_err(|_| Error::InvalidApiKeyFormat)?;

    Ok((api_key_id, hash))
}

fn extract_api_key(req: &Request<Body>) -> Option<String> {
    // Check the query string first
    let query = serde_urlencoded::from_str::<ApiQueryString>(req.uri().query().unwrap_or_default());

    if let Ok(query) = query {
        event!(Level::DEBUG, key=%query.api_key, "Got key from query string");
        return Some(query.api_key);
    }

    // And then for a Bearer token
    req.headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|h| h.split_once(' '))
        .filter(|&(auth_type, _)| auth_type == "Bearer")
        .map(|(_, token)| {
            event!(Level::DEBUG, key=%token, "Got key from auth header");
            token.to_string()
        })
}

#[derive(Deserialize)]
struct ApiQueryString {
    api_key: String,
}

#[async_trait]
pub trait ApiKeyStore: Clone + Send + Sync + 'static {
    type FetchData: Send + Sync + 'static;
    type NewData;
    type Error: From<crate::Error> + IntoResponse + Send + 'static;

    async fn lookup_api_key(
        &self,
        key_id: Uuid,
        hash: Hash,
    ) -> Result<Self::FetchData, Self::Error>;
    async fn create_api_key(&self, key: ApiKeyData, data: Self::NewData)
        -> Result<(), Self::Error>;
    async fn disable_api_key(&self, key_id: Uuid) -> Result<(), Self::Error>;

    fn api_key_prefix(&self) -> &'static str;
}

#[derive(Clone)]
pub struct ApiKeyManager<Store: ApiKeyStore> {
    pub store: Store,
}

impl<Store: ApiKeyStore> std::fmt::Debug for ApiKeyManager<Store> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKeyManager").finish_non_exhaustive()
    }
}

impl<Store: ApiKeyStore> ApiKeyManager<Store> {
    async fn handle_api_key(&self, key: &str) -> Result<Store::FetchData, Store::Error> {
        let (api_key_id, hash) = decode_key(&self.store, key)?;
        event!(Level::DEBUG, ?hash, ?api_key_id, "checking key");
        self.store.lookup_api_key(api_key_id, hash).await
    }

    #[instrument(level = "DEBUG")]
    pub async fn get_api_key(
        &self,
        req: &Request<Body>,
    ) -> Result<Option<Store::FetchData>, Store::Error> {
        event!(Level::DEBUG, "Fetching api key");
        if let Some(key) = extract_api_key(req) {
            let auth = self.handle_api_key(key.borrow()).await?;
            return Ok(Some(auth));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_variables)]
    use std::collections::HashMap;

    use assert_matches::assert_matches;
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{header::AUTHORIZATION, Request},
        response::IntoResponse,
    };
    use chrono::{TimeZone, Utc};

    use super::{decode_key, ApiKeyData, ApiKeyStore};
    use crate::Error;

    #[derive(Copy, Clone, Debug)]
    enum TestKeyStoreError {
        Error,
        KeyNotFound,
        HashMismatch,
    }
    impl IntoResponse for TestKeyStoreError {
        fn into_response(self) -> axum::response::Response {
            todo!()
        }
    }

    impl From<crate::Error> for TestKeyStoreError {
        fn from(_: crate::Error) -> Self {
            Self::Error
        }
    }

    #[derive(Clone, Default)]
    struct TestKeyStore {
        keys: HashMap<uuid::Uuid, ApiKeyData>,
    }

    #[async_trait]
    impl ApiKeyStore for TestKeyStore {
        type NewData = ();
        type FetchData = ();

        type Error = TestKeyStoreError;

        async fn lookup_api_key(
            &self,
            key_id: uuid::Uuid,
            hash: super::Hash,
        ) -> Result<Self::FetchData, Self::Error> {
            let data = self
                .keys
                .get(&key_id)
                .ok_or(TestKeyStoreError::KeyNotFound)?;
            if data.hash == hash {
                Ok(())
            } else {
                Err(TestKeyStoreError::HashMismatch)
            }
        }

        async fn create_api_key(
            &self,
            key: ApiKeyData,
            data: Self::NewData,
        ) -> Result<(), Self::Error> {
            todo!()
        }

        async fn disable_api_key(&self, key: uuid::Uuid) -> Result<(), Self::Error> {
            todo!()
        }

        fn api_key_prefix(&self) -> &'static str {
            "tskey"
        }
    }

    #[test]
    fn valid_key() -> Result<(), Error> {
        let test_store = TestKeyStore::default();
        let data = ApiKeyData::new(&test_store, Utc.ymd(3000, 1, 1).and_hms(0, 0, 0));

        println!("key data {:?}", data.key);

        let (api_key_id, hash) = decode_key(&test_store, &data.key).unwrap();
        assert_eq!(api_key_id, data.id, "id");
        assert_eq!(hash, data.hash, "hash");
        Ok(())
    }

    #[test]
    fn bad_key() -> Result<(), Error> {
        let test_store = TestKeyStore::default();
        let data = ApiKeyData::new(&test_store, Utc.ymd(3000, 1, 1).and_hms(0, 0, 0));

        // Alter the key.
        let mut key = data.key;
        key.pop();
        key.push('a');

        let (api_key_id, hash) = decode_key(&test_store, &key).unwrap();
        assert_eq!(api_key_id, data.id, "id");
        assert_ne!(hash, data.hash, "hash");
        Ok(())
    }

    #[test]
    fn bad_prefix() {
        let test_store = TestKeyStore::default();
        let data = ApiKeyData::new(&test_store, Utc.ymd(3000, 1, 1).and_hms(0, 0, 0));
        let bad_key = format!("a{}", &data.key[1..]);

        decode_key(&test_store, &bad_key).expect_err("bad prefix");
    }

    #[test]
    fn bad_length() {
        let test_store = TestKeyStore::default();
        let data = ApiKeyData::new(&test_store, Utc.ymd(3000, 1, 1).and_hms(0, 0, 0));

        let mut key = String::from(&data.key);
        key.push('a');
        decode_key(&test_store, &key).expect_err("length too high");

        key.pop();
        key.pop();
        decode_key(&test_store, &key).expect_err("length too low");
    }

    #[tokio::test]
    async fn key_from_query_string() {
        let test_store = TestKeyStore::default();
        let key = "tskey.njklsefnjksed";
        let req = Request::builder()
            .uri(&format!("http://localhost/api/tasks?api_key={}", key))
            .body(Body::empty())
            .expect("Creating request");
        let found = super::extract_api_key(&req);
        assert_matches!(found, Some(key));
    }

    #[tokio::test]
    async fn key_from_bearer() {
        let test_store = TestKeyStore::default();
        let key = "tskey.njklsefnjksed";
        let req = Request::builder()
            .uri("http://localhost/api/tasks")
            .header(AUTHORIZATION, format!("Bearer {}", key))
            .body(Body::empty())
            .expect("Creating request");
        let found = super::extract_api_key(&req);
        assert_matches!(found, Some(key));
    }

    #[tokio::test]
    async fn lookup_api_key() {
        let mut test_store = TestKeyStore::default();
        let data = ApiKeyData::new(&test_store, Utc.ymd(3000, 1, 1).and_hms(0, 0, 0));
        test_store.keys.insert(data.id, data.clone());

        let (id, hash) = decode_key(&test_store, &data.key).unwrap();
        test_store.lookup_api_key(id, hash).await.unwrap();
    }
}
