use std::borrow::Cow;

use axum::{
    http::{header::AUTHORIZATION, Request, StatusCode},
    response::{IntoResponse, Response},
};
use tower_cookies::Cookies;

pub const INVALID_MESSAGE_BODY: &str = "401 Unauthorized";
pub fn invalid_message() -> Response {
    // Intentionally vague error message
    (StatusCode::UNAUTHORIZED, INVALID_MESSAGE_BODY).into_response()
}

pub fn extract_bearer_auth_value<B>(req: &Request<B>) -> Result<Option<Cow<str>>, Response> {
    match req.headers().get(AUTHORIZATION) {
        None => Ok(None),
        Some(value) => {
            let (auth_type, token) = value
                .to_str()
                .map_err(|_| invalid_message())?
                .split_once(' ')
                .ok_or_else(invalid_message)?;

            if auth_type != "Bearer" {
                return Err(invalid_message());
            }

            Ok(Some(Cow::Borrowed(token)))
        }
    }
}
