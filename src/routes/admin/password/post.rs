use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Extension, Form,
};
use axum_extra::extract::SignedCookieJar;
use cookie::Cookie;
use secrecy::{ExposeSecret, Secret};

use crate::{
    authentication::{validate_credentials, AuthError, Credentials, UserId},
    routes::admin::dashboard::{get_username, AdminDashboardError},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    State(pool): State<sqlx::PgPool>,
    Extension(user_id): Extension<UserId>,
    signed_jar: SignedCookieJar,
    Form(form): Form<FormData>,
) -> Result<axum::response::Response, AdminDashboardError> {
    let user_id = *user_id;

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        return Ok((
            signed_jar.add(Cookie::new(
                "_flash",
                "You entered two different new passwords - the field values must match.",
            )),
            Redirect::to("/admin/password"),
        )
            .into_response());
    }

    let username = get_username(user_id, &pool)
        .await
        .map_err(AdminDashboardError::UnexpectedError)?;

    let credentials = Credentials {
        username,
        password: form.current_password,
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => Ok((
                signed_jar.add(Cookie::new("_flash", "The current password is incorrect.")),
                Redirect::to("/admin/password"),
            )
                .into_response()),
            AuthError::UnexpectedError(e) => Err(AdminDashboardError::UnexpectedError(e)),
        };
    }
    match crate::authentication::change_password(user_id, form.new_password, &pool).await {
        Ok(_) => Ok((
            signed_jar.add(Cookie::new("_flash", "Your password has been changed.")),
            Redirect::to("/admin/password"),
        )
            .into_response()),
        Err(e) => Err(AdminDashboardError::UnexpectedError(e)),
    }
}
