use axum::{
    extract::{Query, State},
    response::IntoResponse,
};

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(skip(parameters, pool), name = "Confirm a pending subscriber")]
pub async fn confirm(
    State(pool): State<sqlx::PgPool>,
    parameters: Query<Parameters>,
) -> impl IntoResponse {
    let id = match get_subscriber_id_from_token(&pool, &parameters.subscription_token).await {
        Ok(id) => id,
        Err(_) => return axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    };

    match id {
        None => axum::http::StatusCode::UNAUTHORIZED,
        Some(subscriber_id) => {
            if confirm_subscriber(&pool, subscriber_id).await.is_err() {
                return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
            }
            axum::http::StatusCode::OK
        }
    }
}

#[tracing::instrument(skip(subscriber_id, pool), name = "Mark subscriber as confirmed")]
pub async fn confirm_subscriber(
    pool: &sqlx::PgPool,
    subscriber_id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(skip(subscription_token, pool), name = "Get subscriber_id from token")]
pub async fn get_subscriber_id_from_token(
    pool: &sqlx::PgPool,
    subscription_token: &str,
) -> Result<Option<uuid::Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1",
        subscription_token,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}
