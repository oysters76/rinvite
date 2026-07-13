use async_trait::async_trait;
use uuid::Uuid;

use super::super::error::DomainError;
use super::super::event::{Event, EventUpdate, NewEvent};
use super::super::guest::{Guest, GuestUpdate, NewGuest, RsvpStatus};
use super::super::model::User;

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
    /// Create an account and email a verification link. No token is issued —
    /// the user must verify before they can log in.
    async fn signup(&self, email: &str, password: &str) -> Result<(), DomainError>;
    /// Authenticate. Fails with `EmailNotVerified` until the address is verified.
    async fn login(&self, email: &str, password: &str) -> Result<AuthToken, DomainError>;
    /// Confirm an email-verification token, activating the account.
    async fn verify_email(&self, token: &str) -> Result<(), DomainError>;
    /// Re-send the verification email. Always succeeds (no account enumeration);
    /// a mail only goes out if the address exists and is still unverified.
    async fn resend_verification(&self, email: &str) -> Result<(), DomainError>;
    /// The authenticated user's own record (for a "current user" endpoint).
    async fn me(&self, user_id: Uuid) -> Result<User, DomainError>;
}

/// Non-billing "billing" use case: a user asks the app owner to upgrade their
/// plan. There is no self-serve payment — this just notifies the owner.
#[async_trait]
pub trait BillingService: Send + Sync {
    /// Email the app owner that this user would like to upgrade.
    async fn request_upgrade(&self, user_id: Uuid) -> Result<(), DomainError>;
}

/// Owner-facing use cases. Every method is scoped to `owner_id`; a caller can
/// only ever touch their own events and guests.
#[async_trait]
pub trait EventService: Send + Sync {
    async fn create_event(&self, owner_id: Uuid, details: NewEvent) -> Result<Event, DomainError>;
    async fn list_events(&self, owner_id: Uuid) -> Result<Vec<Event>, DomainError>;
    async fn get_event(&self, owner_id: Uuid, event_id: Uuid) -> Result<Event, DomainError>;
    async fn update_event(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        update: EventUpdate,
    ) -> Result<Event, DomainError>;
    async fn delete_event(&self, owner_id: Uuid, event_id: Uuid) -> Result<(), DomainError>;
    async fn add_guest(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        details: NewGuest,
    ) -> Result<Guest, DomainError>;
    async fn list_guests(&self, owner_id: Uuid, event_id: Uuid) -> Result<Vec<Guest>, DomainError>;
    async fn get_guest(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        guest_id: Uuid,
    ) -> Result<Guest, DomainError>;
    async fn update_guest(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        guest_id: Uuid,
        update: GuestUpdate,
    ) -> Result<Guest, DomainError>;
    async fn delete_guest(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        guest_id: Uuid,
    ) -> Result<(), DomainError>;
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

    /// Bulk: one multi-page PDF with a card per print-channel guest.
    async fn render_print_batch(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
    ) -> Result<Vec<u8>, DomainError>;

    /// Bulk: deliver every e-invite-channel guest's link sequentially, and
    /// report per-guest success/failure.
    async fn send_einvite_batch(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
    ) -> Result<BatchSendReport, DomainError>;
}

/// Outcome of one guest's delivery in a bulk send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendStatus {
    Sent,
    Failed,
}

impl SendStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SendStatus::Sent => "sent",
            SendStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SendResult {
    pub guest_id: Uuid,
    pub guest_name: String,
    pub status: SendStatus,
    /// Error detail when `status` is `Failed`.
    pub detail: Option<String>,
}

/// Summary of a bulk e-invite send.
#[derive(Debug, Clone)]
pub struct BatchSendReport {
    pub total: usize,
    pub sent: usize,
    pub failed: usize,
    pub results: Vec<SendResult>,
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
