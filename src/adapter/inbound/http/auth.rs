use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};

use super::{ApiError, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
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
