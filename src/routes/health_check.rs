use axum::http;

#[tracing::instrument(name = "Sending health check result")]
pub async fn healt_check() -> http::StatusCode {
    http::StatusCode::OK
}
