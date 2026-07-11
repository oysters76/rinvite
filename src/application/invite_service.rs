use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::error::DomainError;
use crate::domain::port::inbound::{InviteService, InviteView};
use crate::domain::port::outbound::{Clock, EventRepository, GuestRepository};

/// Implements the public, token-based `InviteService`. The invite token is the
/// only capability required — no user auth. Resolves the guest, loads its
/// event, and applies the RSVP rules from `Guest::respond`.
pub struct InviteServiceImpl {
    events: Arc<dyn EventRepository>,
    guests: Arc<dyn GuestRepository>,
    clock: Arc<dyn Clock>,
}

impl InviteServiceImpl {
    pub fn new(
        events: Arc<dyn EventRepository>,
        guests: Arc<dyn GuestRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            events,
            guests,
            clock,
        }
    }
}

#[async_trait]
impl InviteService for InviteServiceImpl {
    async fn view_invite(&self, token: &str) -> Result<InviteView, DomainError> {
        let guest = self
            .guests
            .find_by_token(token)
            .await?
            .ok_or_else(|| DomainError::NotFound("invite".to_owned()))?;
        let event = self
            .events
            .find(guest.event_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("event".to_owned()))?;

        let rsvp_closed = self.clock.now().date_naive() > event.rsvp_by;

        Ok(InviteView {
            guest_name: guest.name.clone(),
            max_party_size: guest.max_party_size,
            rsvp_status: guest.rsvp_status,
            party_size: guest.party_size,
            rsvp_closed,
            event,
        })
    }

    async fn submit_rsvp(
        &self,
        token: &str,
        attending: bool,
        party_size: u16,
    ) -> Result<(), DomainError> {
        let mut guest = self
            .guests
            .find_by_token(token)
            .await?
            .ok_or_else(|| DomainError::NotFound("invite".to_owned()))?;
        let event = self
            .events
            .find(guest.event_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("event".to_owned()))?;

        guest.respond(attending, party_size, self.clock.now(), event.rsvp_by)?;
        self.guests.update_rsvp(&guest).await?;
        Ok(())
    }
}
