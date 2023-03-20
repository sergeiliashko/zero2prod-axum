use axum::http;

pub async fn healt_check() -> http::StatusCode{
    http::StatusCode::OK
}
