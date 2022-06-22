use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponseData {
    error: ErrorDetails,
}

#[derive(Debug, Serialize)]
struct ErrorDetails {
    details: String,
}

impl ErrorResponseData {
    pub fn new(message: String) -> ErrorResponseData {
        ErrorResponseData {
            error: ErrorDetails { details: message },
        }
    }
}
