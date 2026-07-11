use axum::{
    Json, Router,
    extract::{Path, State},
    response::Html,
    routing::get,
};
use serde::{Deserialize, Serialize};

use super::html::render_invite_page;
use super::{ApiError, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/invite/{token}", get(view))
        .route("/invite/{token}/rsvp", axum::routing::post(rsvp))
}

/// Public e-invite web page — no auth, the token is the capability.
async fn view(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Html<String>, ApiError> {
    let view = state.invites.view_invite(&token).await?;
    Ok(Html(render_invite_page(&view, &token)))
}

#[derive(Deserialize)]
struct RsvpRequest {
    attending: bool,
    party_size: u16,
}

#[derive(Serialize)]
struct RsvpResponse {
    ok: bool,
}

async fn rsvp(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(body): Json<RsvpRequest>,
) -> Result<Json<RsvpResponse>, ApiError> {
    state
        .invites
        .submit_rsvp(&token, body.attending, body.party_size)
        .await?;
    Ok(Json(RsvpResponse { ok: true }))
}
