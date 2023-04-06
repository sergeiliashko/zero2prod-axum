use anyhow::Context;
use axum::{
    extract::State,
    //    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    Extension,
    Form,
};
use axum_extra::extract::SignedCookieJar;
use cookie::Cookie;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
//use base64::Engine;

use crate::{
    //authentication::{Credentials, UserId},
    authentication::UserId,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    routes::error_chain_fmt,
};

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}
#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::UnexpectedError(_) => {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            Self::AuthError(_) => {
                let mut response = Response::default();
                response.headers_mut().insert(
                    axum::http::header::WWW_AUTHENTICATE,
                    axum::http::HeaderValue::from_str(r#"Basic realm="publish""#).unwrap(),
                );
                *response.status_mut() = axum::http::StatusCode::UNAUTHORIZED;
                response
            }
        }
    }
}

//fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
//    let header_value = headers
//        .get("Authorization")
//        .context("The 'Authorization' header was missing")?
//        .to_str()
//        .context("The 'Authorization' header was not a valid UTF8 string.")?;
//    let base64encoded_segment = header_value
//        .strip_prefix("Basic ")
//        .context("The authorization scheme was not 'Basic'.")?;
//    let decoded_bytes = base64::engine::general_purpose::STANDARD
//        .decode(base64encoded_segment)
//        .context("Failed to base64-decode 'Basic' credentials.")?;
//    let decoded_credentials = String::from_utf8(decoded_bytes)
//        .context("The decoded credential string is not valid UTF8.")?;
//
//    let mut credentials = decoded_credentials.splitn(2, ':');
//    let username = credentials
//        .next()
//        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
//        .to_string();
//    let password = credentials
//        .next()
//        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
//        .to_string();
//    Ok(Credentials {
//        username,
//        password: secrecy::Secret::new(password),
//    })
//}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip_all,
    fields(user_id=%&*user_id),
    err(Debug),
    ret
)]
pub async fn publish_newsletter(
    State(pool): State<sqlx::PgPool>,
    Extension(user_id): Extension<UserId>,
    //headers: HeaderMap,
    signed_jar: SignedCookieJar,
    Form(form): Form<BodyData>,
) -> Result<Response, PublishError> {
    let BodyData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form;

    let idempotency_key: IdempotencyKey = idempotency_key.try_into()?;

    let mut transaction = match try_processing(&pool, &idempotency_key, *user_id).await? {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            return Ok((
                signed_jar.add(Cookie::new(
                    "_flash",
                    "The newsletter issue has been accepted - emails will go out shortly.",
                )),
                saved_response,
            )
                .into_response())
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &title, &text_content, &html_content)
        .await
        .context("Failed to store newsletter issue details")?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")?;

    //let subscribers = get_confirmed_subscribers(&pool).await?;
    //for subscriber in subscribers {
    //    match subscriber {
    //        Ok(subscriber) => {
    //            email_client
    //                .send_email(&subscriber.email, &title, &html_content, &text_content)
    //                .await
    //                .with_context(|| {
    //                    format!("Failed to send newsletter issue to {}", subscriber.email)
    //                })?;
    //        }
    //        Err(error) => {
    //            tracing::warn!(
    //                error.cause_chain = ?error,
    //                "Skipping a confirmed subscriber. \
    //                Their stored contact details are invalid",
    //            );
    //        }
    //    }
    //}

    let response = (
        signed_jar.add(Cookie::new(
            "_flash",
            "The newsletter issue has been accepted - emails will go out shortly.",
        )),
        Redirect::to("/admin/newsletter"),
    )
        .into_response();

    let response = save_response(transaction, &idempotency_key, *user_id, response).await?;
    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content
    )
    .execute(transaction)
    .await?;

    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    )
    .execute(transaction)
    .await?;
    Ok(())
}
