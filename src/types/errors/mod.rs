// Copyright 2022 the homieflow authors.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

mod auth;
mod internal;
mod oauth;
mod token;

pub use auth::Error as AuthError;
pub use internal::Error as InternalError;
pub use oauth::Error as OAuthError;
pub use token::Error as TokenError;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, thiserror::Error)]
#[serde(
    tag = "error",
    content = "error_description",
    rename_all = "snake_case"
)]
pub enum ServerError {
    #[error("internal error: {0}")]
    Internal(#[from] InternalError),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("auth error: {0}")]
    Auth(#[from] AuthError),
    #[error("oauth error: {0}")]
    OAuth(#[from] OAuthError),
}

impl axum::response::IntoResponse for ServerError {
    type Body = axum::body::Full<hyper::body::Bytes>;

    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> http::Response<Self::Body> {
        use http::StatusCode;
        let status = match self {
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Auth(ref err) => match err {
                AuthError::InvalidAuthorizationHeader(_) => StatusCode::UNAUTHORIZED,
                AuthError::InvalidToken(_) => StatusCode::UNAUTHORIZED,
                AuthError::InvalidGoogleJwt(_) => StatusCode::UNAUTHORIZED,
                AuthError::InvalidCsrfToken => StatusCode::UNAUTHORIZED,
            },
            Self::OAuth(_) => StatusCode::BAD_REQUEST,
        };
        let mut response = axum::Json(self).into_response();
        *response.status_mut() = status;

        response
    }
}

impl From<TokenError> for ServerError {
    fn from(e: TokenError) -> Self {
        Self::Auth(e.into())
    }
}

impl From<askama::Error> for InternalError {
    fn from(e: askama::Error) -> Self {
        Self::Template(e.to_string())
    }
}

impl From<askama::Error> for ServerError {
    fn from(e: askama::Error) -> Self {
        Self::Internal(e.into())
    }
}
