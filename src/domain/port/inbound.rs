use async_trait::async_trait;
use uuid::Uuid;

use super::super::error::DomainError;
use super::super::event::{Event, NewEvent};
use super::super::guest::{Guest, NewGuest, RsvpStatus};

/// Returned to a client after a successful signup/login.
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub value: String,
}

// ===== Inbound ports (driving side) ========================================
// What the outside world is allowed to ask of our application. HTTP handlers,
// a CLI, or a message consumer all call *this* trait and nothing deeper.

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn signup(&self, email: &str, password: &str) -> Result<AuthToken, DomainError>;
    async fn login(&self, email: &str, password: &str) -> Result<AuthToken, DomainError>;
}

/// Owner-facing use cases. Every method is scoped to `owner_id`; a caller can
/// only ever touch their own events and guests.
#[async_trait]
pub trait EventService: Send + Sync {
    async fn create_event(&self, owner_id: Uuid, details: NewEvent) -> Result<Event, DomainError>;
    async fn list_events(&self, owner_id: Uuid) -> Result<Vec<Event>, DomainError>;
    async fn get_event(&self, owner_id: Uuid, event_id: Uuid) -> Result<Event, DomainError>;
    async fn add_guest(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        details: NewGuest,
    ) -> Result<Guest, DomainError>;
    async fn list_guests(&self, owner_id: Uuid, event_id: Uuid) -> Result<Vec<Guest>, DomainError>;
    /// Bytes of a printable PDF invitation for one guest.
    async fn render_invite_pdf(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        guest_id: Uuid,
    ) -> Result<Vec<u8>, DomainError>;
    /// Deliver a guest's e-invite via the configured sender, returning the
    /// invite URL that was sent.
    async fn send_invite(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        guest_id: Uuid,
    ) -> Result<String, DomainError>;
}

/// What the public e-invite page needs to render itself.
#[derive(Debug, Clone)]
pub struct InviteView {
    pub event: Event,
    pub guest_name: String,
    pub max_party_size: u16,
    pub rsvp_status: RsvpStatus,
    pub party_size: Option<u16>,
    /// True once the RSVP deadline has passed (form should be read-only).
    pub rsvp_closed: bool,
}

/// Public, capability-based use cases: anyone holding a guest's invite token
/// can view the invitation and submit that guest's RSVP.
#[async_trait]
pub trait InviteService: Send + Sync {
    async fn view_invite(&self, token: &str) -> Result<InviteView, DomainError>;
    async fn submit_rsvp(
        &self,
        token: &str,
        attending: bool,
        party_size: u16,
    ) -> Result<(), DomainError>;
}
