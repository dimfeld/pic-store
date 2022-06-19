use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Datalog parse failure: {0}")]
    DatalogTokenError(#[from] biscuit_auth::error::Token),

    #[error("Invalid format for {0}")]
    IdParseError(&'static str),

    #[error("Missing credentials")]
    MissingCredentials,
}
