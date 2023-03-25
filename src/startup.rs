use axum::{
    body::{Body, BoxBody},
    http::{HeaderName, Request, Response},
    routing::{get, post},
    Router,
};
use std::time::Duration;
use tracing::Span;

use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

use crate::{routes, email_client::EmailClient};

// In axum, we have only one state type
#[derive(Clone)]
struct AppState {
    email_client: EmailClient,
    connection_pool: sqlx::PgPool,
}
impl axum::extract::FromRef<AppState> for EmailClient {
    fn from_ref(app_state: &AppState) -> EmailClient {
        app_state.email_client.clone()
    }
}
impl axum::extract::FromRef<AppState> for sqlx::PgPool {
    fn from_ref(app_state: &AppState) -> sqlx::PgPool {
        app_state.connection_pool.clone()
    }
}

#[allow(dead_code)]
pub async fn app(connection_pool: sqlx::PgPool, email_client: EmailClient) -> Router {
    let x_request_id = HeaderName::from_static("x-request-id");
    let state = AppState {
        email_client,
        connection_pool
    };

    Router::new()
        .route("/health_check", get(routes::healt_check))
        .route("/subscriptions", post(routes::subscribe))
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(
            // from https://docs.rs/tower-http/0.2.5/tower_http/request_id/index.html#using-trace
            tower::ServiceBuilder::new()
                .layer(SetRequestIdLayer::new(
                    x_request_id.clone(),
                    MakeRequestUuid,
                ))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|_request: &Request<Body>| {
                            tracing::info_span!(
                                "http-request",
                                status_code = tracing::field::Empty,
                                request_id = format!("{}", uuid::Uuid::new_v4()),
                            )
                        })
                        .on_response(
                            |response: &Response<BoxBody>, _latency: Duration, span: &Span| {
                                span.record(
                                    "status_code",
                                    &tracing::field::display(response.status()),
                                );
                                tracing::info!("response generated")
                            },
                        ), //.make_span_with(
                           //    DefaultMakeSpan::new()
                           //        .include_headers(true)
                           //        .level(tracing::Level::INFO),
                           //)
                           //.on_response(DefaultOnResponse::new().include_headers(true))
                )
                //.set_x_request_id(MakeRequestUuid)
                .layer(PropagateRequestIdLayer::new(x_request_id)),
        )
        //.propagate_x_request_id())
        //.with_state(connection_pool)
        .with_state(state)
        //.with_state(email_client)
}
