use axum::{extract::FromRequestParts, http::request::Parts};
use axum_sessions::extractors::WritableSession;
use uuid::Uuid;
use serde_json::error::Error;
use axum::response::IntoResponse;

use axum::async_trait;

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";
    pub fn renew(&mut self) { 
        self.0.regenerate()
    }
    pub fn insert_user_id(& mut self, user_id: Uuid) -> Result<(), Error> { 
        self.0.insert(Self::USER_ID_KEY, user_id)
    }
    pub fn get_user_id(&self) -> Option<Uuid> {
        self.0.get::<Uuid>(Self::USER_ID_KEY)
    }
    pub fn log_out(mut self) {
        self.0.destroy()
    }
}
pub struct TypedSession(WritableSession);

#[async_trait]
impl<S> FromRequestParts<S> for TypedSession
where
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Ok(session) = WritableSession::from_request_parts(parts, _state).await{
            Ok(TypedSession(session))
        } else {
            Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "were not able to extract writablesession").into_response())
        }

    }
}
