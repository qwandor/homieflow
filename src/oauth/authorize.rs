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

use super::verify_redirect_uri;
use super::AuthorizationRequestQuery;
use crate::types::errors::InternalError;
use crate::types::errors::OAuthError;
use crate::types::errors::ServerError;
use crate::State;
use askama::Template;
use axum::extract::Extension;
use axum::extract::Query;
use axum::response::Html;
use http::HeaderMap;
use url::Url;

#[derive(Template)]
#[template(path = "authorize.html")]
struct AuthorizeTemplate {
    client_id: String,
    redirect_uri: Url,
    state: String,
    base_url: Url,
    google_login_client_id: Option<String>,
}

#[tracing::instrument(name = "Authorization", skip(state), err)]
pub async fn handle(
    Extension(state): Extension<State>,
    Query(request): Query<AuthorizationRequestQuery>,
    headers: HeaderMap,
) -> Result<Html<String>, ServerError> {
    let google_config = state
        .config
        .google
        .as_ref()
        .ok_or_else(|| InternalError::Other("Google Home API not configured".to_string()))?;
    if *request.client_id != *google_config.client_id {
        return Err(OAuthError::InvalidClient(Some(String::from("invalid client id"))).into());
    }
    verify_redirect_uri(&request.redirect_uri, &google_config.project_id)
        .map_err(|err| OAuthError::InvalidRequest(Some(err.to_string())))?;

    let template = AuthorizeTemplate {
        client_id: request.client_id.to_owned(),
        redirect_uri: request.redirect_uri.to_owned(),
        state: request.state.to_owned(),
        base_url: state.config.get_base_url(),
        google_login_client_id: state
            .config
            .logins
            .google
            .as_ref()
            .map(|c| c.client_id.to_owned()),
    };
    Ok(Html(template.render()?))
}
