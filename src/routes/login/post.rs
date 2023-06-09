use axum::extract::{Form, State};
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};

use crate::session_state::TypedSession;
use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::error_chain_fmt,
};

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
        Redirect::to("/login").into_response()
    }
}

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: secrecy::Secret<String>,
}

#[tracing::instrument(
    skip(form, pool, signed_jar, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty),
    err(Debug),
)]
pub async fn login(
    State(pool): State<sqlx::PgPool>,
    mut session: TypedSession,
    signed_jar: SignedCookieJar,
    Form(form): Form<FormData>,
) -> Result<Redirect, (SignedCookieJar, LoginError)> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            session.insert_user_id(user_id).map_err(|e| {
                login_redirect_on_err(LoginError::UnexpectedError(e.into()), signed_jar)
            })?;
            Ok(Redirect::to("/admin/dashboard"))
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            Err(login_redirect_on_err(e, signed_jar))
        }
    }
}

fn login_redirect_on_err(
    e: LoginError,
    signed_jar: SignedCookieJar,
) -> (SignedCookieJar, LoginError) {
    (signed_jar.add(Cookie::new("_flash", e.to_string())), e)
}
