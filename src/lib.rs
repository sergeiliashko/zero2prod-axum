use axum::{
    http,
    routing::get,
    Router};

#[allow(dead_code)]
pub async fn app() -> Router {
    Router::new()
        .route("/health_check", get(healt_check))
}

async fn healt_check() -> http::StatusCode{
    http::StatusCode::OK
}
