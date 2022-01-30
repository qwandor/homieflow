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

mod execute;
mod homie;
mod query;
mod sync;

use crate::extractors::UserID;
use crate::types::errors::ServerError;
use crate::State;
use axum::extract::Extension;
use axum::Json;
use google_smart_home::Request;
use google_smart_home::RequestInput;
use google_smart_home::Response;

#[tracing::instrument(name = "GHome", skip(state), err)]
pub async fn handle(
    Extension(state): Extension<State>,
    UserID(user_id): UserID,
    Json(request): Json<Request>,
) -> Result<Json<Response>, ServerError> {
    let input = request.inputs.first().unwrap();

    let body: Response = match input {
        RequestInput::Sync => Response::Sync(google_smart_home::sync::response::Response {
            request_id: request.request_id,
            payload: sync::handle(state, user_id).await?,
        }),
        RequestInput::Query(payload) => {
            Response::Query(google_smart_home::query::response::Response {
                request_id: request.request_id,
                payload: query::handle(state, user_id, payload).await?,
            })
        }
        RequestInput::Execute(payload) => {
            Response::Execute(google_smart_home::execute::response::Response {
                request_id: request.request_id,
                payload: execute::handle(state, user_id, payload).await?,
            })
        }
        RequestInput::Disconnect => todo!(),
    };

    Ok(Json(body))
}
