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

use serde::Deserialize;
use serde::Serialize;

#[allow(clippy::enum_variant_names)]
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
