use axum::{
    http::{header::AUTHORIZATION, Request, StatusCode},
    response::{IntoResponse, Response},
};

fn invalid_message() -> Response {
    // Intentionally vague error message
    (StatusCode::UNAUTHORIZED, "401 Unauthorized").into_response()
}

pub fn extract_bearer_auth_value<B>(req: &Request<B>) -> Result<Option<String>, Response> {
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

            Ok(Some(token.to_string()))
        }
    }
}
