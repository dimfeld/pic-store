use std::borrow::Cow;

use axum::{
    http::{header::AUTHORIZATION, Request, StatusCode},
    response::{IntoResponse, Response},
};
use tower_cookies::Cookies;

pub fn invalid_message() -> Response {
    // Intentionally vague error message
    (StatusCode::UNAUTHORIZED, "401 Unauthorized").into_response()
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

pub fn extract_from_cookie<'a, B>(req: &'a Request<B>, cookie_name: &str) -> Option<Cow<'a, str>> {
    // Get the Cookies if it's already there, or parse it ourselves otherwise.
    req.extensions()
        .get::<Cookies>()
        .and_then(|cookies| cookies.get(cookie_name))
        .map(|cookie| {
            cookie
                .value_raw()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| Cow::Owned(cookie.value().to_string()))
        })
}
