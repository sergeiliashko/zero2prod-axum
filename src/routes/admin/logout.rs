use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::SignedCookieJar;
use cookie::Cookie;

use crate::session_state::TypedSession;

pub async fn log_out(
    session: TypedSession,
    signed_jar: SignedCookieJar,
)-> impl IntoResponse {
    if session.get_user_id().is_none() {
        Redirect::to("/login").into_response()
    } else {
        session.log_out();
        (signed_jar.add(
            Cookie::build("_flash", "You have successfully logged out.")
                .path("/")
                .finish()
        ),
            Redirect::to("/login")
        ).into_response()
    }
}
