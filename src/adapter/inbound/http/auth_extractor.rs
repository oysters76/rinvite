use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use uuid::Uuid;

use super::{ApiError, AppState};
use crate::domain::error::DomainError;

/// Extractor that authenticates the caller from a `Authorization: Bearer <jwt>`
/// header and yields their user id. Any handler that takes `AuthUser` is
/// automatically protected — a missing/invalid token becomes a `401`.
pub struct AuthUser(pub Uuid);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(DomainError::Unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(DomainError::Unauthorized)?;
        let user_id = state.verifier.verify(token.trim())?;
        Ok(AuthUser(user_id))
    }
}
