use axum::{middleware::Next, http::Request, response::{Redirect, IntoResponse}};
use std::ops::Deref;

use crate::session_state::TypedSession;

#[derive(Copy, Clone, Debug)] 
pub struct UserId(uuid::Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f) 
    }
}

impl Deref for UserId {
    type Target = uuid::Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// The deadlock with async_sessions requires drop session https://github.com/maxcountryman/axum-sessions/issues/13
pub async fn reject_anonymous_users<B>(session: TypedSession, mut request: Request<B>, next: Next<B>) -> axum::response::Response {
    match session.get_user_id() { 
        Some(user_id) => {
            request.extensions_mut().insert(UserId(user_id));
            drop(session);
            next.run(request).await
        },
        None => {
            //let e = anyhow::anyhow!("The user has not logged in");
            Redirect::to("/login").into_response()
        } }
}
