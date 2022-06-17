use std::any::Any;

use axum::{
    body::Body,
    http::{header, Response, StatusCode},
};

pub fn handle_panic(production: bool, err: Box<dyn Any + Send + 'static>) -> Response<Body> {
    let body = if production {
        serde_json::json!({ "error": { "details": "Server error" } })
    } else {
        let details = if let Some(s) = err.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = err.downcast_ref::<&str>() {
            s.to_string()
        } else {
            "Unknown panic message".to_string()
        };

        serde_json::json!({
            "error": {
                "kind": "panic",
                "details": details,
            }
        })
    };

    let body = serde_json::to_string(&body).unwrap();

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}
