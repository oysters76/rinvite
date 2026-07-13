use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::auth_extractor::AuthUser;
use super::{ApiError, AppState};
use crate::domain::error::DomainError;
use crate::domain::event::{Event, EventUpdate, NewEvent};
use crate::domain::guest::{Guest, GuestUpdate, InviteChannel, NewGuest};
use crate::domain::port::inbound::BatchSendReport;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events", post(create_event).get(list_events))
        .route(
            "/events/{id}",
            get(get_event).patch(update_event).delete(delete_event),
        )
        .route("/events/{id}/guests", post(add_guest).get(list_guests))
        .route(
            "/events/{id}/guests/{gid}",
            get(get_guest).patch(update_guest).delete(delete_guest),
        )
        .route("/events/{id}/guests/{gid}/invite.pdf", get(invite_pdf))
        .route("/events/{id}/guests/{gid}/send", post(send_invite))
        .route("/events/{id}/invites/print.pdf", get(print_batch))
        .route("/events/{id}/invites/send", post(send_batch))
}

// ----- DTOs -----------------------------------------------------------------

#[derive(Deserialize)]
struct CreateEventRequest {
    bride_name: String,
    bride_family_name: String,
    groom_name: String,
    groom_family_name: String,
    event_date: NaiveDate,
    start_time: NaiveTime,
    end_time: NaiveTime,
    hall_name: String,
    venue_name: String,
    rsvp_by: NaiveDate,
}

impl From<CreateEventRequest> for NewEvent {
    fn from(r: CreateEventRequest) -> Self {
        NewEvent {
            bride_name: r.bride_name,
            bride_family_name: r.bride_family_name,
            groom_name: r.groom_name,
            groom_family_name: r.groom_family_name,
            event_date: r.event_date,
            start_time: r.start_time,
            end_time: r.end_time,
            hall_name: r.hall_name,
            venue_name: r.venue_name,
            rsvp_by: r.rsvp_by,
        }
    }
}

#[derive(Serialize)]
struct EventResponse {
    id: Uuid,
    bride_name: String,
    bride_family_name: String,
    groom_name: String,
    groom_family_name: String,
    event_date: NaiveDate,
    start_time: NaiveTime,
    end_time: NaiveTime,
    hall_name: String,
    venue_name: String,
    rsvp_by: NaiveDate,
}

impl From<Event> for EventResponse {
    fn from(e: Event) -> Self {
        Self {
            id: e.id,
            bride_name: e.bride_name,
            bride_family_name: e.bride_family_name,
            groom_name: e.groom_name,
            groom_family_name: e.groom_family_name,
            event_date: e.event_date,
            start_time: e.start_time,
            end_time: e.end_time,
            hall_name: e.hall_name,
            venue_name: e.venue_name,
            rsvp_by: e.rsvp_by,
        }
    }
}

#[derive(Deserialize)]
struct CreateGuestRequest {
    name: String,
    /// "print" or "einvite".
    channel: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    phone: Option<String>,
    max_party_size: u16,
}

/// Map the request channel string to the domain enum, treating an unknown value
/// as caller error (400) rather than corrupt data.
fn parse_channel(s: &str) -> Result<InviteChannel, DomainError> {
    match s {
        "print" => Ok(InviteChannel::Print),
        "einvite" => Ok(InviteChannel::EInvite),
        other => Err(DomainError::InvalidInput(format!(
            "channel must be 'print' or 'einvite', got '{other}'"
        ))),
    }
}

#[derive(Serialize)]
struct GuestResponse {
    id: Uuid,
    name: String,
    channel: String,
    email: Option<String>,
    phone: Option<String>,
    max_party_size: u16,
    rsvp_status: String,
    party_size: Option<u16>,
    /// Shareable e-invite link for this guest.
    invite_url: String,
}

fn guest_response(g: Guest, base_url: &str) -> GuestResponse {
    let invite_url = format!("{base_url}/i/{}", g.invite_token);
    GuestResponse {
        id: g.id,
        name: g.name,
        channel: g.channel.as_str().to_owned(),
        email: g.email,
        phone: g.phone,
        max_party_size: g.max_party_size,
        rsvp_status: g.rsvp_status.as_str().to_owned(),
        party_size: g.party_size,
        invite_url,
    }
}

// ----- Handlers -------------------------------------------------------------

async fn create_event(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateEventRequest>,
) -> Result<Response, ApiError> {
    let event = state.events.create_event(owner_id, body.into()).await?;
    Ok((StatusCode::CREATED, Json(EventResponse::from(event))).into_response())
}

async fn list_events(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<EventResponse>>, ApiError> {
    let events = state.events.list_events(owner_id).await?;
    Ok(Json(events.into_iter().map(EventResponse::from).collect()))
}

async fn get_event(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventResponse>, ApiError> {
    let event = state.events.get_event(owner_id, event_id).await?;
    Ok(Json(EventResponse::from(event)))
}

/// Partial event update — only the fields present are changed.
#[derive(Deserialize)]
struct UpdateEventRequest {
    bride_name: Option<String>,
    bride_family_name: Option<String>,
    groom_name: Option<String>,
    groom_family_name: Option<String>,
    event_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    end_time: Option<NaiveTime>,
    hall_name: Option<String>,
    venue_name: Option<String>,
    rsvp_by: Option<NaiveDate>,
}

impl From<UpdateEventRequest> for EventUpdate {
    fn from(r: UpdateEventRequest) -> Self {
        EventUpdate {
            bride_name: r.bride_name,
            bride_family_name: r.bride_family_name,
            groom_name: r.groom_name,
            groom_family_name: r.groom_family_name,
            event_date: r.event_date,
            start_time: r.start_time,
            end_time: r.end_time,
            hall_name: r.hall_name,
            venue_name: r.venue_name,
            rsvp_by: r.rsvp_by,
        }
    }
}

async fn update_event(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    Json(body): Json<UpdateEventRequest>,
) -> Result<Json<EventResponse>, ApiError> {
    let event = state
        .events
        .update_event(owner_id, event_id, body.into())
        .await?;
    Ok(Json(EventResponse::from(event)))
}

async fn delete_event(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.events.delete_event(owner_id, event_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn add_guest(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    Json(body): Json<CreateGuestRequest>,
) -> Result<Response, ApiError> {
    let details = NewGuest {
        name: body.name,
        channel: parse_channel(&body.channel)?,
        email: body.email,
        phone: body.phone,
        max_party_size: body.max_party_size,
    };
    let guest = state.events.add_guest(owner_id, event_id, details).await?;
    Ok((
        StatusCode::CREATED,
        Json(guest_response(guest, &state.public_base_url)),
    )
        .into_response())
}

async fn list_guests(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<Vec<GuestResponse>>, ApiError> {
    let guests = state.events.list_guests(owner_id, event_id).await?;
    Ok(Json(
        guests
            .into_iter()
            .map(|g| guest_response(g, &state.public_base_url))
            .collect(),
    ))
}

async fn get_guest(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path((event_id, guest_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<GuestResponse>, ApiError> {
    let guest = state.events.get_guest(owner_id, event_id, guest_id).await?;
    Ok(Json(guest_response(guest, &state.public_base_url)))
}

/// Partial guest update — only the fields present are changed. Contact fields
/// present with a value set it; absent leaves it unchanged.
#[derive(Deserialize)]
struct UpdateGuestRequest {
    name: Option<String>,
    channel: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    max_party_size: Option<u16>,
}

async fn update_guest(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path((event_id, guest_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateGuestRequest>,
) -> Result<Json<GuestResponse>, ApiError> {
    let update = GuestUpdate {
        name: body.name,
        channel: body.channel.map(|c| parse_channel(&c)).transpose()?,
        email: body.email.map(Some),
        phone: body.phone.map(Some),
        max_party_size: body.max_party_size,
    };
    let guest = state
        .events
        .update_guest(owner_id, event_id, guest_id, update)
        .await?;
    Ok(Json(guest_response(guest, &state.public_base_url)))
}

async fn delete_guest(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path((event_id, guest_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    state
        .events
        .delete_guest(owner_id, event_id, guest_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn invite_pdf(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path((event_id, guest_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, ApiError> {
    let bytes = state
        .events
        .render_invite_pdf(owner_id, event_id, guest_id)
        .await?;
    Ok(([(header::CONTENT_TYPE, "application/pdf")], bytes).into_response())
}

#[derive(Serialize)]
struct SendResponse {
    sent: bool,
    invite_url: String,
}

async fn send_invite(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path((event_id, guest_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<SendResponse>, ApiError> {
    let invite_url = state
        .events
        .send_invite(owner_id, event_id, guest_id)
        .await?;
    Ok(Json(SendResponse {
        sent: true,
        invite_url,
    }))
}

// ----- Bulk endpoints -------------------------------------------------------

/// One merged PDF with a card per print-channel guest.
async fn print_batch(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let bytes = state.events.render_print_batch(owner_id, event_id).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"invitations.pdf\"",
            ),
        ],
        bytes,
    )
        .into_response())
}

#[derive(Serialize)]
struct SendResultDto {
    guest_id: Uuid,
    guest_name: String,
    status: String,
    detail: Option<String>,
}

#[derive(Serialize)]
struct BatchReportDto {
    total: usize,
    sent: usize,
    failed: usize,
    results: Vec<SendResultDto>,
}

impl From<BatchSendReport> for BatchReportDto {
    fn from(r: BatchSendReport) -> Self {
        Self {
            total: r.total,
            sent: r.sent,
            failed: r.failed,
            results: r
                .results
                .into_iter()
                .map(|s| SendResultDto {
                    guest_id: s.guest_id,
                    guest_name: s.guest_name,
                    status: s.status.as_str().to_owned(),
                    detail: s.detail,
                })
                .collect(),
        }
    }
}

/// Sequentially deliver every e-invite guest's link; report per-guest results.
async fn send_batch(
    AuthUser(owner_id): AuthUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<BatchReportDto>, ApiError> {
    let report = state.events.send_einvite_batch(owner_id, event_id).await?;
    Ok(Json(report.into()))
}
