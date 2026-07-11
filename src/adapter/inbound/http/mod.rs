use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tower_http::timeout::TimeoutLayer;

use crate::domain::error::DomainError;
use crate::domain::port::inbound::{AuthService, EventService, InviteService};
use crate::domain::port::outbound::TokenVerifier;

pub mod auth;
pub mod auth_extractor;
pub mod events;
pub mod html;
pub mod invites;

/// Bodies are tiny JSON objects; cap well below axum's 2 MB default so an
/// oversized request is rejected before we read it.
const MAX_BODY_BYTES: usize = 8 * 1024;
/// Ceiling on how long a single request may take.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Shared state handed to every handler. It holds the INBOUND PORTS as trait
/// objects (handlers depend on abstractions), plus the token verifier used by
/// the `AuthUser` extractor and the public base URL for building invite links.
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<dyn AuthService>,
    pub events: Arc<dyn EventService>,
    pub invites: Arc<dyn InviteService>,
    pub verifier: Arc<dyn TokenVerifier>,
    pub public_base_url: String,
    /// The e-invite HTML template, loaded once at startup.
    pub einvite_template: Arc<str>,
}

/// Compose every route group and apply the shared middleware.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .merge(auth::router())
        .merge(events::router())
        .merge(invites::router())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            REQUEST_TIMEOUT,
        ))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

// ----- Error mapping: the ONE place that maps DomainError -> HTTP status -----

pub struct ApiError(pub DomainError);

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        ApiError(e)
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            DomainError::EmailAlreadyExists => (StatusCode::CONFLICT, self.0.to_string()),
            DomainError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            DomainError::Unauthorized => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            DomainError::NotFound(_) => (StatusCode::NOT_FOUND, self.0.to_string()),
            DomainError::RsvpClosed => (StatusCode::CONFLICT, self.0.to_string()),
            DomainError::PartySizeExceeded => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.0.to_string())
            }
            // Safe to echo back: the message describes the caller's own input.
            DomainError::InvalidInput(_) => (StatusCode::BAD_REQUEST, self.0.to_string()),
            // Never leak internal detail to clients on 5xx.
            DomainError::Repository(_)
            | DomainError::Hashing(_)
            | DomainError::TokenCreation(_)
            | DomainError::Pdf(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_owned(),
            ),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}
