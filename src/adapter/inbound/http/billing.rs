use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::Serialize;

use super::auth_extractor::AuthUser;
use super::{ApiError, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/billing/upgrade-request", post(request_upgrade))
        // Public: the SPA reads the business contact address for the dialog.
        .route("/config", axum::routing::get(config))
}

/// The authenticated user asks the app owner to upgrade their plan. Sends the
/// owner an email; returns 204. Plan changes are applied manually by the owner.
async fn request_upgrade(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    state.billing.request_upgrade(user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct ConfigResponse {
    contact_email: String,
}

/// Public client config: currently just the business contact address shown in
/// the "limit reached" dialog.
async fn config(State(state): State<AppState>) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        contact_email: state.contact_email.clone(),
    })
}
