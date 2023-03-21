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

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields( subscriber_email = %form.email, subscriber_name = %form.name) 
)]
pub async fn subscribe(
    State(pool): State<sqlx::postgres::PgPool>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    match insert_subscriber(&pool, &form).await {
        Ok(_) => axum::http::StatusCode::OK,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, pool)
)]
pub async fn insert_subscriber(
    pool: &sqlx::postgres::PgPool,
    form: &FormData) -> Result<(),sqlx::Error> {

    sqlx::query!(r#" INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4) "#,
        Uuid::new_v4(),
        form.email, 
        form.name,
        Utc::now())
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e})?;
    // Using the `?` operator to return early
    // if the function failed, returning a sqlx::Error 
    // We will talk about error handling in depth later!
    Ok(())
}
