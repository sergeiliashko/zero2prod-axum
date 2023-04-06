//use axum::{response::{IntoResponse, Response}, http::status::StatusCode};
//
//use crate::routes::error_chain_fmt;
//
//struct OpaqueError(anyhow::Error);
//
//impl<E> From<E> for OpaqueError
//where
//    E: Into<anyhow::Error>,
//{
//    fn from(err: E) -> Self {
//        Self(err.into())
//    }
//}
//
////impl std::fmt::Debug for OpaqueError {
////    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
////        error_chain_fmt(self.into(), f)
////    }
////}
//
//impl IntoResponse for OpaqueError {
//    fn into_response(self) -> Response {
//        format!("Something went wrong: {}", self.0.to_string()).into_response()
//    }
//}
//
//fn e400(e: OpaqueError) -> impl IntoResponse
//{
//    (
//        axum::http::StatusCode::BAD_REQUEST,
//        e
//    )
//}
