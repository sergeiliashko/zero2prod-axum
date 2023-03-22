use axum::{
    routing::{get, post},
    Router, 
    http::{HeaderName, Request, Response },
    body::{BoxBody, Body},
};
use std::time::Duration;
use tracing::Span;

use tower_http::request_id::{ SetRequestIdLayer, PropagateRequestIdLayer, MakeRequestUuid};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;


use crate::routes;

#[allow(dead_code)]
pub async fn app(connection_pool: sqlx::PgPool) -> Router {

    let x_request_id = HeaderName::from_static("x-request-id");
    
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
                        .on_response(|response: &Response<BoxBody>, _latency: Duration, span: &Span| {
                            span.record("status_code", &tracing::field::display(response.status()));
                            tracing::info!("response generated")
                        })
                        //.make_span_with(
                        //    DefaultMakeSpan::new()
                        //        .include_headers(true)
                        //        .level(tracing::Level::INFO),
                        //)
                        //.on_response(DefaultOnResponse::new().include_headers(true))
                )
                //.set_x_request_id(MakeRequestUuid)
                .layer(PropagateRequestIdLayer::new(x_request_id)))
                //.propagate_x_request_id())
        .with_state(connection_pool)
}
