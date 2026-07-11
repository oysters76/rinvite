use async_trait::async_trait;

use crate::domain::error::DomainError;
use crate::domain::guest::Guest;
use crate::domain::port::outbound::InviteSender;

/// No-op `InviteSender`: the invite URL is surfaced through the API and shared
/// by hand (WhatsApp, email, ...). This is the placeholder the real bulk/
/// per-guest senders will replace — the port keeps that change out of the core.
pub struct ManualSender;

#[async_trait]
impl InviteSender for ManualSender {
    async fn send(&self, _guest: &Guest, _invite_url: &str) -> Result<(), DomainError> {
        Ok(())
    }
}
