use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, thiserror::Error)]
pub enum Error {
    /// `Authorization` header has invalid syntax
    #[error("invalid authorization header: {0}")]
    InvalidAuthorizationHeader(String),
    /// Client sent invalid token
    #[error("invalid token: {0}")]
    InvalidToken(#[from] super::token::Error),
    /// Invalid Google JWT
    #[error("invalid Google JWT: {0}")]
    InvalidGoogleJwt(String),
    /// The CSRF token cookie was missing, or didn't match the token in the request.
    #[error("Missing or invalid CSRF token")]
    InvalidCsrfToken,
}
