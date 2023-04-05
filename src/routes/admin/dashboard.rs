use axum::{response::{IntoResponse, Html}, extract::State, Extension};
use anyhow::Context;

use crate::{routes::error_chain_fmt, authentication::UserId};

// Return an opaque 500 while preserving the error's root cause for logging. 
//fn e500<T>(e: T) -> actix_web::Error
//where
//    T: std::fmt::Debug + std::fmt::Display + 'static 
//{
//    actix_web::error::ErrorInternalServerError(e) 
//}

#[derive(thiserror::Error)]
pub enum AdminDashboardError {
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for AdminDashboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for AdminDashboardError {
    fn into_response(self) -> axum::response::Response {
        axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

#[tracing::instrument(
    skip(pool, user_id),
    err(Debug),
)]
pub async fn admin_dashboard(
    Extension(user_id): Extension<UserId>,
    State(pool): State<sqlx::PgPool>,
) -> Result<axum::response::Response, AdminDashboardError>   {

    let username = get_username(*user_id, &pool)
        .await
        .map_err(|e| AdminDashboardError::UnexpectedError(e.into()))?;

    Ok(Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Admin dashboard</title>
</head>
<body>
    <p>Welcome {username}!</p>
    <p>Available actions:</p>
    <ol>
        <li><a href="/admin/password">Change password</a></li>
        <li>
          <form name="logoutForm" action="/admin/logout" method="post">
            <input type="submit" value="Logout">
        </form>
        </li>
    </ol>
</body> </html>"#,
    )).into_response())
}

#[tracing::instrument(name = "Get username", skip(pool))] 
pub async fn get_username(
    user_id: uuid::Uuid,
    pool: &sqlx::PgPool
) -> Result<String, anyhow::Error> {
    let row = sqlx::query!( r#"
        SELECT username 
        FROM users
        WHERE user_id = $1
        "#,
        user_id, 
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;
    Ok(row.username)
}
