use chrono::Utc;
use serde::Deserialize;
use axum::{Form, response::IntoResponse, extract::State};
use uuid::Uuid;

#[derive(Debug)]
#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(
    State(pool): State<sqlx::postgres::PgPool>,
    Form(form): Form<FormData>,) -> impl IntoResponse {
    match sqlx::query!( r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(), form.email, form.name, Utc::now())
        .execute(&pool)
        .await {
            Ok(_) => axum::http::StatusCode::OK,
            Err(e) => {
                    println!("Failed to execute query: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                }
        }

}
