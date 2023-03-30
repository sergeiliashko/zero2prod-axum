use axum::response::{IntoResponse, Redirect};
use axum::extract::{Form, State};
use hmac::{Hmac, Mac};
use secrecy:: ExposeSecret;

use crate::routes::{authentication::{Credentials, validate_credentials, AuthError}, error_chain_fmt};
use crate::startup::HmacSecret;

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f) 
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> axum::response::Response {
        self.to_string().into_response()
    }
}

#[derive(serde::Deserialize)]
pub struct FormData{
    username: String,
    password: secrecy::Secret<String>,
}

#[tracing::instrument(
skip(form, pool, hmac_secret),
fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(pool): State<sqlx::PgPool>,
    State(hmac_secret): State<HmacSecret>,
    Form(form): Form<FormData>
) -> impl IntoResponse{
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current()
                .record("user_id", &tracing::field::display(&user_id));
            Redirect::to("/")
        },
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) =>  LoginError::UnexpectedError(e.into()) ,
            };
            let query_string = format!(
                "error={}",
                urlencoding::Encoded::new(e.to_string())
            );
            let hmac_tag = {
                let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
                    hmac_secret.0.expose_secret().as_bytes()
                ).unwrap();
                mac.update(query_string.as_bytes());
                mac.finalize().into_bytes()
            };
            Redirect::to(&format!("/login?{}&tag={:x}",query_string,hmac_tag))
        }
    }
}

