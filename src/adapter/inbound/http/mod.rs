use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::{Method, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tower_http::cors::{AllowOrigin, CorsLayer};
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
        // Liveness probe for load balancers / uptime monitors (App Platform
        // health checks hit this). Public, no auth, no state.
        .route("/health", axum::routing::get(|| async { "ok" }))
        .merge(auth::router())
        .merge(events::router())
        .merge(invites::router())
        .layer(cors_layer_from_env())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            REQUEST_TIMEOUT,
        ))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

/// CORS for browser SPAs. `CORS_ALLOWED_ORIGINS` (comma-separated) restricts to
/// an allowlist; unset means allow any origin — safe here because auth is
/// Bearer-token based (no cookies/credentials).
fn cors_layer_from_env() -> CorsLayer {
    let origins = match std::env::var("CORS_ALLOWED_ORIGINS") {
        Ok(raw) => {
            let list: Vec<_> = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse().ok())
                .collect();
            AllowOrigin::list(list)
        }
        Err(_) => AllowOrigin::any(),
    };
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
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
