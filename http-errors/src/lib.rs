use serde::Serialize;
use std::borrow::Cow;

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
        ErrorResponseData {
            error: ErrorDetails {
                kind: kind.into(),
                message: message.into(),
            },
        }
    }
}
