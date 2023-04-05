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
//use base64::Engine;

use crate::{
    //authentication::{Credentials, UserId},
    authentication::UserId,
    domain::SubscriberEmail,
    email_client::EmailClient,
    routes::error_chain_fmt,
};

//#[derive(serde::Deserialize)]
//pub struct BodyData {
//    title: String,
//    content: Content,
//}

//#[derive(serde::Deserialize)]
//pub struct Content {
//    html: String,
//    text: String,
//}

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    html: String,
    text: String,
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
    name = "Publish a newsletter",
    skip(pool, email_client, body),
    //fields( subscriber_email = %form.email, subscriber_name = %form.name),
    err(Debug),
    ret
)]
pub async fn publish_newsletter(
    State(pool): State<sqlx::PgPool>,
    State(email_client): State<EmailClient>,
    Extension(_user_id): Extension<UserId>,
    //headers: HeaderMap,
    signed_jar: SignedCookieJar,
    Form(body): Form<BodyData>,
) -> Result<impl IntoResponse, PublishError> {
    //let credentials = basic_authentication(&headers).map_err(PublishError::AuthError)?;
    //tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    //let user_id = validate_credentials(credentials, &pool)
    //    .await
    //    .map_err(|e| match e {
    //        AuthError::InvalidCredentials(_) => PublishError::AuthError(e.into()),
    //        AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into()),
    //    })?;
    //tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    let subscribers = get_confirmed_subscribers(&pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &body.title, &body.html, &body.text)
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                );
            }
        }
    }

    Ok((
        signed_jar.add(Cookie::new("_flash", "Newsletter was sent successfully.")),
        Redirect::to("/admin/newsletter"),
    ))
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &sqlx::PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed' "#,
    )
    .fetch_all(pool)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();
    Ok(confirmed_subscribers)
}
