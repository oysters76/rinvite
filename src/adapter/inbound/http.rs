use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use tower_http::timeout::TimeoutLayer;

use crate::domain::error::DomainError;
use crate::domain::port::inbound::AuthService;

/// Auth payloads are tiny JSON objects; cap the body well below axum's 2 MB
/// default so an oversized request is rejected before we read it.
const MAX_BODY_BYTES: usize = 8 * 1024;
/// Ceiling on how long a single request may take (an argon2 hash is well under
/// a second, so this only trips on pathological load).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Shared state handed to every handler. It holds the INBOUND PORT as a trait
/// object, so handlers depend on the abstraction, not the concrete service.
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<dyn AuthService>,
}

/// Build the router for the auth endpoints. To add a new endpoint you add a
/// `.route(...)` line here plus a handler below.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            REQUEST_TIMEOUT,
        ))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

// ----- Request / response DTOs (transport shapes, kept out of the domain) ----

#[derive(Deserialize)]
struct Credentials {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
}

// ----- Handlers: thin. Parse input -> call inbound port -> shape output -----

async fn signup(
    State(state): State<AppState>,
    Json(body): Json<Credentials>,
) -> Result<Response, ApiError> {
    let token = state.auth.signup(&body.email, &body.password).await?;
    Ok((
        StatusCode::CREATED,
        Json(AuthResponse { token: token.value }),
    )
        .into_response())
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<Credentials>,
) -> Result<Response, ApiError> {
    let token = state.auth.login(&body.email, &body.password).await?;
    Ok((StatusCode::OK, Json(AuthResponse { token: token.value })).into_response())
}

// ----- Error mapping: the ONE place that maps DomainError -> HTTP status -----

struct ApiError(DomainError);

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
            // Safe to echo back: the message describes the caller's own input.
            DomainError::InvalidInput(_) => (StatusCode::BAD_REQUEST, self.0.to_string()),
            // Never leak internal detail to clients on 5xx.
            DomainError::Repository(_)
            | DomainError::Hashing(_)
            | DomainError::TokenCreation(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_owned(),
            ),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}
