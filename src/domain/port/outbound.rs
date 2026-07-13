use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::super::error::DomainError;
use super::super::event::Event;
use super::super::guest::Guest;
use super::super::model::User;

// ===== Outbound ports (driven side) ========================================
// What our application needs *from* the outside world. Each of these is
// implemented by one or more adapters (Postgres, in-memory, argon2, jwt, ...).

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
}

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, plaintext: &str) -> Result<String, DomainError>;
    async fn verify(&self, plaintext: &str, hash: &str) -> Result<bool, DomainError>;

    /// Verify against a fixed internal hash to equalize timing when no user
    /// exists — defeats account enumeration via response latency. The concrete
    /// dummy hash lives in the adapter, so the application never learns the
    /// hash format.
    async fn verify_dummy(&self, plaintext: &str) -> Result<(), DomainError>;
}

pub trait TokenIssuer: Send + Sync {
    fn issue(&self, user_id: Uuid) -> Result<String, DomainError>;
}

/// The other half of `TokenIssuer`: turn a bearer token back into the user id
/// it was issued for (or reject it). Used by the auth middleware.
pub trait TokenVerifier: Send + Sync {
    fn verify(&self, token: &str) -> Result<Uuid, DomainError>;
}

#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn save(&self, event: &Event) -> Result<(), DomainError>;
    async fn find(&self, id: Uuid) -> Result<Option<Event>, DomainError>;
    async fn list_by_owner(&self, owner_id: Uuid) -> Result<Vec<Event>, DomainError>;
    async fn update(&self, event: &Event) -> Result<(), DomainError>;
    /// Delete the event; its guests are removed too (DB cascade / in-memory).
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}

#[async_trait]
pub trait GuestRepository: Send + Sync {
    async fn save(&self, guest: &Guest) -> Result<(), DomainError>;
    async fn find(&self, id: Uuid) -> Result<Option<Guest>, DomainError>;
    async fn find_by_token(&self, token: &str) -> Result<Option<Guest>, DomainError>;
    async fn list_by_event(&self, event_id: Uuid) -> Result<Vec<Guest>, DomainError>;
    /// Persist an RSVP change (status / party size / responded_at).
    async fn update_rsvp(&self, guest: &Guest) -> Result<(), DomainError>;
    /// Persist edits to the guest's details (name/channel/contact/max party).
    async fn update(&self, guest: &Guest) -> Result<(), DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}

/// Renders printable invitation PDFs for guests of an event.
pub trait InvitePdfRenderer: Send + Sync {
    /// A single-page PDF for one guest.
    fn render(&self, event: &Event, guest: &Guest) -> Result<Vec<u8>, DomainError>;
    /// One multi-page PDF with a page per guest, in order.
    fn render_all(&self, event: &Event, guests: &[Guest]) -> Result<Vec<u8>, DomainError>;
}

/// Source of "now" — injected so RSVP-deadline logic and timestamps are
/// deterministic in tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Delivery seam for e-invites. Implemented by a dispatcher that routes each
/// guest to WhatsApp or email; the event is passed so adapters can render rich
/// messages (couple, date, venue).
#[async_trait]
pub trait InviteSender: Send + Sync {
    async fn send(&self, event: &Event, guest: &Guest, invite_url: &str)
    -> Result<(), DomainError>;
}

/// Sends a transactional email (HTML + plain-text alternative).
#[async_trait]
pub trait EmailClient: Send + Sync {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html: &str,
        text: &str,
    ) -> Result<(), DomainError>;
}

/// Sends a WhatsApp message body to a phone number.
#[async_trait]
pub trait WhatsAppClient: Send + Sync {
    async fn send_whatsapp(&self, to_phone: &str, body: &str) -> Result<(), DomainError>;
}
