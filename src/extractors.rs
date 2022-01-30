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

use crate::config::server::Secrets;
use crate::types::errors::AuthError;
use crate::types::errors::ServerError;
use crate::types::errors::TokenError;
use crate::types::token::AccessTokenPayload;
use crate::types::token::RefreshTokenPayload;
use crate::types::token::Token;
use crate::types::user;
use crate::State;
use async_trait::async_trait;
use axum::body::Body;
use jsonwebtoken::TokenData;
use serde::de;
use serde::ser;

pub struct UserID(pub user::ID);

#[async_trait]
impl axum::extract::FromRequest<Body> for UserID {
    type Rejection = ServerError;

    async fn from_request(
        req: &mut axum::extract::RequestParts<Body>,
    ) -> Result<Self, Self::Rejection> {
        let AccessToken(access_token) = AccessToken::from_request(req).await?;
        Ok(Self(access_token.claims.sub))
    }
}

pub struct RefreshToken(pub TokenData<RefreshTokenPayload>);
pub struct AccessToken(pub TokenData<AccessTokenPayload>);

async fn from_request<P>(
    req: &mut axum::extract::RequestParts<Body>,
    get_key_fn: impl FnOnce(&Secrets) -> &str,
) -> Result<TokenData<P>, AuthError>
where
    P: ser::Serialize + de::DeserializeOwned,
{
    let state: &State = req.extensions().unwrap().get().unwrap();
    let header_str = req
        .headers()
        .unwrap()
        .get(http::header::AUTHORIZATION)
        .ok_or(TokenError {
            description: "MissingHeader".to_string(),
        })?
        .to_str()
        .map_err(|err| AuthError::InvalidAuthorizationHeader(err.to_string()))?;

    let (schema, token) = header_str
        .split_once(' ')
        .ok_or_else(|| AuthError::InvalidAuthorizationHeader(String::from("invalid syntax")))?;

    if schema != "Bearer" {
        return Err(AuthError::InvalidAuthorizationHeader(schema.to_string()));
    }

    Ok(Token::<P>::decode(
        get_key_fn(&state.config.secrets).as_bytes(),
        token,
    )?)
}

#[async_trait]
impl axum::extract::FromRequest<Body> for RefreshToken {
    type Rejection = ServerError;

    async fn from_request(
        req: &mut axum::extract::RequestParts<Body>,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(
            from_request(req, |secrets| &secrets.refresh_key).await?,
        ))
    }
}

#[async_trait]
impl axum::extract::FromRequest<Body> for AccessToken {
    type Rejection = ServerError;

    async fn from_request(
        req: &mut axum::extract::RequestParts<Body>,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(
            from_request(req, |secrets| &secrets.access_key).await?,
        ))
    }
}
