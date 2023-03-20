use axum::{
    routing::{get, post},
    Router, 
};

use crate::routes;

#[allow(dead_code)]
pub async fn app(connection_pool: sqlx::PgPool) -> Router {
    Router::new()
        .route("/health_check", get(routes::healt_check))
        .route("/subscriptions", post(routes::subscribe))
        .with_state(connection_pool)
}
