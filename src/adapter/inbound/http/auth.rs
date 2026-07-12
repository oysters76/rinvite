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

#[derive(Serialize)]
struct UserResponse {
    id: Uuid,
    email: String,
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
    }))
}
