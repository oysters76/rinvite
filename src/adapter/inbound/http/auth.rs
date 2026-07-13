use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::auth_extractor::AuthUser;
use super::{ApiError, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/verify", post(verify))
        .route("/auth/resend-verification", post(resend_verification))
        .route("/auth/me", get(me))
}

#[derive(Deserialize)]
struct Credentials {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
}

/// Signup no longer returns a token: the user must verify their email first.
#[derive(Serialize)]
struct SignupResponse {
    verification_required: bool,
}

async fn signup(
    State(state): State<AppState>,
    Json(body): Json<Credentials>,
) -> Result<Response, ApiError> {
    state.auth.signup(&body.email, &body.password).await?;
    Ok((
        StatusCode::CREATED,
        Json(SignupResponse {
            verification_required: true,
        }),
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

#[derive(Deserialize)]
struct VerifyRequest {
    token: String,
}

/// Confirm an email-verification token. 204 on success; 400 if invalid/expired.
async fn verify(
    State(state): State<AppState>,
    Json(body): Json<VerifyRequest>,
) -> Result<StatusCode, ApiError> {
    state.auth.verify_email(&body.token).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct ResendRequest {
    email: String,
}

/// Re-send the verification email. Always 204 (never reveals account state).
async fn resend_verification(
    State(state): State<AppState>,
    Json(body): Json<ResendRequest>,
) -> Result<StatusCode, ApiError> {
    state.auth.resend_verification(&body.email).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct UserResponse {
    id: Uuid,
    email: String,
    plan: String,
    email_verified: bool,
}

/// The authenticated caller's own account (never the password hash).
async fn me(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UserResponse>, ApiError> {
    let user = state.auth.me(user_id).await?;
    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        plan: user.plan.as_str().to_owned(),
        email_verified: user.email_verified,
    }))
}
