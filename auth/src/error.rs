use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid API Key Format")]
    InvalidApiKeyFormat,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Invalid format for {0}")]
    IdParseError(&'static str),

    #[error("Password hasher error: {0}")]
    PasswordHasherError(String),

    #[error("Missing credentials")]
    MissingCredentials,
}
