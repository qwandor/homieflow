pub mod config;
mod extractors;
mod fulfillment;
pub mod homie;
mod oauth;
mod types;

use crate::types::user;
use axum::Router;
use config::server::Config;
use homie_controller::HomieController;
use std::collections::HashMap;
use std::sync::Arc;

async fn health_check() -> &'static str {
    "I'm alive!"
}

#[derive(Clone)]
pub struct State {
    pub config: Arc<Config>,
    pub homie_controllers: Arc<HashMap<user::ID, Arc<HomieController>>>,
}

pub fn app(state: State) -> Router<hyper::Body> {
    use axum::routing::get;
    use axum::routing::post;
    use http::Request;
    use http::Response;
    use hyper::Body;
    use std::time::Duration;
    use tower_http::trace::TraceLayer;
    use tracing::Span;

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
            Router::new().route("/google-home", post(fulfillment::ghome::handle)),
        )
        .layer(axum::AddExtensionLayer::new(state))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<Body>| {
                    tracing::debug_span!(
                        "Request",
                        status_code = tracing::field::Empty,
                        ms = tracing::field::Empty,
                        path = tracing::field::display(request.uri().path()),
                    )
                })
                .on_response(|response: &Response<_>, latency: Duration, span: &Span| {
                    span.record("status_code", &tracing::field::display(response.status()));
                    span.record("ms", &tracing::field::display(latency.as_millis()));

                    tracing::debug!("response processed")
                }),
        )
}
