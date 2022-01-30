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

use crate::types::errors::InternalError;
use crate::types::errors::OAuthError;
use crate::types::errors::ServerError;
use crate::types::token::AccessToken;
use crate::types::token::AccessTokenPayload;
use crate::types::token::AuthorizationCode;
use crate::types::token::RefreshToken;
use crate::types::token::RefreshTokenPayload;
use crate::State;
use axum::extract::Extension;
use axum::extract::Form;
use axum::Json;
use chrono::Duration;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

const GOOGLE_HOME_ACCESS_TOKEN_DURATION_MINUTES: i64 = 10;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "grant_type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Request {
    RefreshToken {
        /// The client ID
        client_id: String,
        /// The client secret
        client_secret: String,
        /// The refresh token previously issued to the client.
        refresh_token: String,
    },

    AuthorizationCode {
        /// The client ID
        client_id: String,
        /// The client secret
        client_secret: String,
        /// This parameter is the authorization code that the client previously received from the authorization server.
        code: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Response {
    /// The access token string as issued by the authorization server.
    pub access_token: String,

    /// The refresh token string as issued by the authorization server.
    pub refresh_token: Option<String>,

    /// The type of token this is, typically just the string “Bearer”.
    pub token_type: TokenType,

    /// If the access token expires, the server should reply with the duration of time the access token is granted for.
    #[serde(with = "crate::types::serde_token_expiration")]
    pub expires_in: Option<Duration>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum TokenType {
    Bearer,
}

async fn on_refresh_token_grant(
    state: State,
    refresh_token: String,
) -> Result<Response, ServerError> {
    let refresh_token = RefreshToken::decode(
        state.config.secrets.refresh_key.as_bytes(),
        &refresh_token,
    )
    .map_err(|err| {
        OAuthError::InvalidGrant(Some(format!("invalid refresh token: {}", err.to_string())))
    })?;

    tracing::info!(user_id = %refresh_token.claims.sub, "Refresh token grant");

    let expires_in = Duration::minutes(GOOGLE_HOME_ACCESS_TOKEN_DURATION_MINUTES);
    let access_token = AccessToken::new(
        state.config.secrets.access_key.as_bytes(),
        AccessTokenPayload {
            sub: refresh_token.claims.sub,
            exp: Utc::now() + expires_in,
        },
    )?;

    Ok(Response {
        access_token: access_token.to_string(),
        token_type: TokenType::Bearer,
        expires_in: Some(expires_in),
        refresh_token: None,
    })
}

async fn on_authorization_code_grant(state: State, code: String) -> Result<Response, ServerError> {
    let code = AuthorizationCode::decode(
        state.config.secrets.authorization_code_key.as_bytes(),
        &code,
    )
    .map_err(|err| {
        OAuthError::InvalidGrant(Some(format!(
            "invalid authorization code: {}",
            err.to_string()
        )))
    })?;

    tracing::info!(user_id = %code.claims.sub, "Authorization code grant");

    let expires_in = Duration::minutes(10);
    let access_token = AccessToken::new(
        state.config.secrets.access_key.as_bytes(),
        AccessTokenPayload {
            sub: code.claims.sub,
            exp: Utc::now() + expires_in,
        },
    )?;

    let refresh_token = RefreshToken::new(
        state.config.secrets.refresh_key.as_bytes(),
        RefreshTokenPayload {
            sub: code.claims.sub,
            exp: None,
        },
    )?;

    Ok(Response {
        access_token: access_token.to_string(),
        refresh_token: Some(refresh_token.to_string()),
        token_type: TokenType::Bearer,
        expires_in: Some(expires_in),
    })
}

#[tracing::instrument(name = "Token", skip(state, request))]
pub async fn handle(
    Extension(state): Extension<State>,
    Form(request): Form<Request>,
) -> Result<Json<Response>, ServerError> {
    let verify_client = |client_id, client_secret| -> Result<(), ServerError> {
        let google_config =
            state.config.google.as_ref().ok_or_else(|| {
                InternalError::Other("Google Home API not configured".to_string())
            })?;
        if client_id != google_config.client_id || client_secret != google_config.client_secret {
            Err(OAuthError::InvalidClient(None).into())
        } else {
            Ok(())
        }
    };

    match request {
        Request::RefreshToken {
            refresh_token,
            client_id,
            client_secret,
            ..
        } => {
            verify_client(client_id, client_secret)?;
            on_refresh_token_grant(state, refresh_token).await
        }
        Request::AuthorizationCode {
            client_id,
            client_secret,
            code,
            ..
        } => {
            verify_client(client_id, client_secret)?;
            on_authorization_code_grant(state, code).await
        }
    }
    .map(Json)
}

// #[cfg(test)]
// mod tests {
//     use crate::types::token::AuthorizationCodePayload;
//
//     use super::*;
//     use crate::test_utils::*;
//
//     #[tokio::test]
//     async fn valid() {
//         let state = get_state();
//         let google_config = state.config.google.as_ref().unwrap();
//
//         let code_payload = AuthorizationCodePayload {
//             sub: rand::random(),
//             exp: Utc::now() + Duration::minutes(10),
//         };
//         let code = AuthorizationCode::new(
//             state.config.secrets.authorization_code_key.as_bytes(),
//             code_payload.clone(),
//         );
//         let response = on_token_grant(
//             Form(Request::AuthorizationCode {
//                 client_id: google_config.client_id.clone(),
//                 client_secret: google_config.client_secret.clone(),
//                 code: code.to_string(),
//             }),
//             state.token_store.clone(),
//             state.config.clone(),
//         )
//         .await
//         .unwrap()
//         .into_inner();
//
//         let response = on_token_grant(
//             Form(Request::RefreshToken {
//                 client_id: google_config.client_id.clone(),
//                 client_secret: google_config.client_secret.clone(),
//                 refresh_token: response
//                     .refresh_token
//                     .expect("authorization grant did not return refresh token"),
//             }),
//             state.token_store.clone(),
//             state.config.clone(),
//         )
//         .await
//         .unwrap()
//         .into_inner();
//
//         let at = AccessToken::decode(
//             state.config.secrets.access_key.as_bytes(),
//             &response.access_token,
//         )
//         .unwrap();
//         assert_eq!(at.sub, code_payload.sub);
//     }
//
//     mod refresh_token_grant {
//         use super::*;
//
//         #[actix_rt::test]
//         async fn valid() {
//             let state = get_state();
//             let google_config = state.config.google.as_ref().unwrap();
//             let refresh_token_payload = RefreshTokenPayload {
//                 sub: rand::random(),
//                 exp: Some(Utc::now() + Duration::minutes(10)),
//                 tid: rand::random(),
//             };
//             let refresh_token = RefreshToken::new(
//                 state.config.secrets.refresh_key.as_bytes(),
//                 refresh_token_payload.clone(),
//             );
//             state
//                 .token_store
//                 .add(&refresh_token.tid, refresh_token.exp.as_ref())
//                 .await
//                 .unwrap();
//             let response = on_token_grant(
//                 Form(Request::RefreshToken {
//                     client_id: google_config.client_id.clone(),
//                     client_secret: google_config.client_secret.clone(),
//                     refresh_token: refresh_token.to_string(),
//                 }),
//                 state.token_store.clone(),
//                 state.config.clone(),
//             )
//             .await
//             .unwrap()
//             .into_inner();
//
//             let at = AccessToken::decode(
//                 state.config.secrets.access_key.as_bytes(),
//                 &response.access_token,
//             )
//             .unwrap();
//             assert_eq!(at.sub, refresh_token_payload.sub);
//         }
//
//         #[actix_rt::test]
//         async fn invalid_client() {
//             let state = get_state();
//             let response = on_token_grant(
//                 Form(Request::RefreshToken {
//                     client_id: String::from("invalid-client-id"),
//                     client_secret: String::from("invalid-client-secret"),
//                     refresh_token: String::from("some-invalid-token"),
//                 }),
//                 state.token_store,
//                 state.config,
//             )
//             .await
//             .unwrap_err();
//             assert!(matches!(response, ResponseError::InvalidClient(..)))
//         }
//     }
//
//     mod authorization_code_grant {
//         use crate::types::token::AuthorizationCodePayload;
//
//         use super::*;
//
//         #[actix_rt::test]
//         async fn valid() {
//             let state = get_state();
//             let google_config = state.config.google.as_ref().unwrap();
//             let code_payload = AuthorizationCodePayload {
//                 sub: rand::random(),
//                 exp: Utc::now() + Duration::minutes(10),
//             };
//             let code = AuthorizationCode::new(
//                 state.config.secrets.authorization_code_key.as_bytes(),
//                 code_payload.clone(),
//             );
//             let response = on_token_grant(
//                 Form(Request::AuthorizationCode {
//                     client_id: google_config.client_id.clone(),
//                     client_secret: google_config.client_secret.clone(),
//                     code: code.to_string(),
//                 }),
//                 state.token_store.clone(),
//                 state.config.clone(),
//             )
//             .await
//             .unwrap()
//             .into_inner();
//
//             let (at, rt) = (response.access_token, response.refresh_token.unwrap());
//             let (at, rt) = (
//                 AccessToken::decode(state.config.secrets.access_key.as_bytes(), &at).unwrap(),
//                 RefreshToken::decode(state.config.secrets.refresh_key.as_bytes(), &rt).unwrap(),
//             );
//             assert_eq!(at.sub, code_payload.sub);
//             assert_eq!(rt.sub, code_payload.sub);
//             assert!(
//                 state.token_store.exists(&rt.tid).await.unwrap(),
//                 "returned refresh token not found in store"
//             );
//         }
//
//         #[actix_rt::test]
//         async fn invalid_client() {
//             let state = get_state();
//             let response = on_token_grant(
//                 Form(Request::AuthorizationCode {
//                     client_id: String::from("invalid-client-id"),
//                     client_secret: String::from("invalid-client-secret"),
//                     code: String::from("some-invalid-token"),
//                 }),
//                 state.token_store,
//                 state.config,
//             )
//             .await
//             .unwrap_err();
//             assert!(matches!(response, ResponseError::InvalidClient(..)))
//         }
//     }
// }
