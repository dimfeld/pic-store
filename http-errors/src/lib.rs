use serde::Serialize;
use std::borrow::Cow;
use tracing::{event, Level};

#[derive(Debug, Serialize)]
pub struct ErrorResponseData {
    error: ErrorDetails,
}

#[derive(Debug, Serialize)]
struct ErrorDetails {
    kind: Cow<'static, str>,
    message: Cow<'static, str>,
}

impl ErrorResponseData {
    pub fn new(
        kind: impl Into<Cow<'static, str>>,
        message: impl Into<Cow<'static, str>>,
    ) -> ErrorResponseData {
        let ret = ErrorResponseData {
            error: ErrorDetails {
                kind: kind.into(),
                message: message.into(),
            },
        };

        event!(Level::ERROR, kind=%ret.error.kind, message=%ret.error.message);

        ret
    }
}
