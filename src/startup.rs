use axum::{
    routing::{get, post},
    Router, 
    http::Request
};
use tower_http::request_id::{ SetRequestIdLayer, PropagateRequestIdLayer, MakeRequestId, RequestId, };
use tower_http::cors::{CorsLayer, Any};
use tower_http::ServiceBuilderExt;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};



use crate::routes;

#[derive(Clone, Copy)]
struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let request_id = uuid::Uuid::new_v4()
            .to_string()
            .parse()
            .unwrap();
        Some(RequestId::new(request_id))
    }
}

#[allow(dead_code)]
pub async fn app(connection_pool: sqlx::PgPool) -> Router {
    Router::new()
        .route("/health_check", get(routes::healt_check))
        .route("/subscriptions", post(routes::subscribe))
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(
            // from https://docs.rs/tower-http/0.2.5/tower_http/request_id/index.html#using-trace
            tower::ServiceBuilder::new()
                .set_x_request_id(MakeRequestUuid)
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(
                            DefaultMakeSpan::new()
                                .include_headers(true)
                                .level(tracing::Level::INFO),
                        )
                        .on_response(DefaultOnResponse::new().include_headers(true)),
                )
                .propagate_x_request_id())
        .with_state(connection_pool)
}
