use std::{borrow::Cow, fmt::Debug, ops::Deref, sync::Arc};

use async_trait::async_trait;
use axum::{
    body::{Body, Bytes, HttpBody},
    extract::{FromRequest, RequestParts},
    http::{header::HeaderName, HeaderValue, Request, StatusCode},
    response::{ErrorResponse, IntoResponse, Response},
    BoxError, Json,
};
use futures::{future::BoxFuture, Future, FutureExt, TryFutureExt};
use serde::Serialize;
use thiserror::Error;
use tower::{Layer, Service};
use uuid::Uuid;

use crate::extract_token::invalid_message;

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
