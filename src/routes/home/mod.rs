use axum::response::IntoResponse;
use axum::response::Html;


pub async fn home() -> impl IntoResponse {
    Html(include_str!("home.html"))
}
