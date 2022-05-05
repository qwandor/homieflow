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

pub mod config;
mod extractors;
mod fulfillment;
pub mod homegraph;
pub mod homie;
mod oauth;
mod types;

use crate::types::user;
use axum::routing::{get, post};
use axum::{AddExtensionLayer, Router};
use config::server::Config;
use homie_controller::HomieController;
use http::{Request, Response};
use hyper::Body;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing::{debug, debug_span, Span};

async fn health_check() -> &'static str {
    "I'm alive!"
}

#[derive(Clone)]
pub struct State {
    pub config: Arc<Config>,
    pub homie_controllers: Arc<HashMap<user::ID, Arc<HomieController>>>,
}

pub fn app(state: State) -> Router<hyper::Body> {
    Router::new()
        .route("/health_check", get(health_check))
        .nest(
            "/oauth",
            Router::new()
                .route("/authorize", get(oauth::authorize::handle))
                .route("/google_login", post(oauth::google_login::handle))
                .route("/token", post(oauth::token::handle)),
        )
        .nest(
            "/fulfillment",
            Router::new().route("/google-home", post(fulfillment::handle)),
        )
        .layer(AddExtensionLayer::new(state))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<Body>| {
                    debug_span!(
                        "Request",
                        status_code = tracing::field::Empty,
                        ms = tracing::field::Empty,
                        path = tracing::field::display(request.uri().path()),
                    )
                })
                .on_response(|response: &Response<_>, latency: Duration, span: &Span| {
                    span.record("status_code", &tracing::field::display(response.status()));
                    span.record("ms", &tracing::field::display(latency.as_millis()));

                    debug!("response processed")
                }),
        )
}
