use axum::response::Html;
use axum::response::IntoResponse;

pub async fn home() -> impl IntoResponse {
    Html(include_str!("home.html"))
}
