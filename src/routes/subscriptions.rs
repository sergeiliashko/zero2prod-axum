use axum::{extract::State, response::IntoResponse, Form};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName};

#[derive(Debug, Deserialize)]
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
    let name = match SubscriberName::parse(form.name) {
        Ok(name) => name,
        Err(_) => return axum::http::StatusCode::BAD_REQUEST,
    };
    let new_subscriber = NewSubscriber {
        email: form.email,
        name
    };
    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => axum::http::StatusCode::OK,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, pool)
)]
pub async fn insert_subscriber(
    pool: &sqlx::postgres::PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#" INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4) "#,
        Uuid::new_v4(),
        new_subscriber.email,
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    // Using the `?` operator to return early
    // if the function failed, returning a sqlx::Error
    // We will talk about error handling in depth later!
    Ok(())
}
