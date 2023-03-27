use axum::{extract::State, response::IntoResponse, Form};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error("{1}")]
    UnexpectedError(#[source] Box<dyn std::error::Error>, String),
}

// We are still using a bespoke implementation of `Debug` // to get a nice report using the error source chain
impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::ValidationError(_) => {
                (axum::http::StatusCode::BAD_REQUEST, format!("{}", self)).into_response()
            }
            Self::UnexpectedError(_,_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("{}", self),
            )
                .into_response(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;
    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields( subscriber_email = %form.email, subscriber_name = %form.name),
    err(Debug),
    ret
)]
pub async fn subscribe(
    State(base_url): State<ApplicationBaseUrl>,
    State(pool): State<sqlx::postgres::PgPool>,
    State(email_client): State<EmailClient>,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, SubscribeError> {
    let new_subscriber = form.try_into().map_err(SubscribeError::ValidationError)?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|e| SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to acquire a Postgres connection from the pool".into(),
        ))?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .map_err(|e| SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to insert new subscriber in the database.".into(),
        ))?;

    let subscription_token = generate_subscription_token();

    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .map_err(|e| SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to store the confirmation token for a new subscriber.".into(),
        ))?;

    transaction
        .commit()
        .await
        .map_err(|e| SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to commit SQL transaction to store a new subscriber.".into(),
        ))?;

    send_confirmation_email(
        &base_url.0,
        &email_client,
        new_subscriber,
        &subscription_token,)
        .await
        .map_err(|e| SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to send a confirmation email.".into(),
        ))?;

    Ok(axum::http::StatusCode::OK)
}

/// Generate a random 25-characters-long case-sensitive subscription token.
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(new_subscriber, email_client)
)]
pub async fn send_confirmation_email(
    base_url: &str,
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let plain_body = format!(
        "Welcome to our newsletter!\n Visti {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br/> \
            Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id) VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#" INSERT INTO subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation') "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    // Using the `?` operator to return early
    // if the function failed, returning a sqlx::Error
    // We will talk about error handling in depth later!
    Ok(subscriber_id)
}
