use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::{Event, NewEvent};
use crate::domain::guest::{Guest, NewGuest};
use crate::domain::port::inbound::EventService;
use crate::domain::port::outbound::{
    Clock, EventRepository, GuestRepository, InvitePdfRenderer, InviteSender,
};
use crate::domain::validation::{validate_event, validate_guest};

/// Implements the owner-facing `EventService`. Every read/write is scoped to
/// the calling `owner_id`; a resource owned by someone else is reported as
/// `NotFound` so we never confirm its existence.
pub struct EventServiceImpl {
    events: Arc<dyn EventRepository>,
    guests: Arc<dyn GuestRepository>,
    pdf: Arc<dyn InvitePdfRenderer>,
    sender: Arc<dyn InviteSender>,
    clock: Arc<dyn Clock>,
    /// Public base URL used to build shareable invite links.
    public_base_url: String,
}

impl EventServiceImpl {
    pub fn new(
        events: Arc<dyn EventRepository>,
        guests: Arc<dyn GuestRepository>,
        pdf: Arc<dyn InvitePdfRenderer>,
        sender: Arc<dyn InviteSender>,
        clock: Arc<dyn Clock>,
        public_base_url: String,
    ) -> Self {
        Self {
            events,
            guests,
            pdf,
            sender,
            clock,
            public_base_url,
        }
    }

    /// Load an event only if it exists and belongs to `owner_id`.
    async fn owned_event(&self, owner_id: Uuid, event_id: Uuid) -> Result<Event, DomainError> {
        match self.events.find(event_id).await? {
            Some(event) if event.owner_id == owner_id => Ok(event),
            _ => Err(DomainError::NotFound("event".to_owned())),
        }
    }

    /// Load a guest, confirming it belongs to the given (already owner-checked)
    /// event.
    async fn guest_of(&self, event_id: Uuid, guest_id: Uuid) -> Result<Guest, DomainError> {
        match self.guests.find(guest_id).await? {
            Some(g) if g.event_id == event_id => Ok(g),
            _ => Err(DomainError::NotFound("guest".to_owned())),
        }
    }
}

#[async_trait]
impl EventService for EventServiceImpl {
    async fn create_event(&self, owner_id: Uuid, details: NewEvent) -> Result<Event, DomainError> {
        validate_event(&details)?;
        let event = Event::new(owner_id, details, self.clock.now());
        self.events.save(&event).await?;
        Ok(event)
    }

    async fn list_events(&self, owner_id: Uuid) -> Result<Vec<Event>, DomainError> {
        self.events.list_by_owner(owner_id).await
    }

    async fn get_event(&self, owner_id: Uuid, event_id: Uuid) -> Result<Event, DomainError> {
        self.owned_event(owner_id, event_id).await
    }

    async fn add_guest(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        details: NewGuest,
    ) -> Result<Guest, DomainError> {
        self.owned_event(owner_id, event_id).await?; // ownership gate
        validate_guest(&details)?;
        let guest = Guest::new(event_id, details, self.clock.now());
        self.guests.save(&guest).await?;
        Ok(guest)
    }

    async fn list_guests(&self, owner_id: Uuid, event_id: Uuid) -> Result<Vec<Guest>, DomainError> {
        self.owned_event(owner_id, event_id).await?; // ownership gate
        self.guests.list_by_event(event_id).await
    }

    async fn render_invite_pdf(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        guest_id: Uuid,
    ) -> Result<Vec<u8>, DomainError> {
        let event = self.owned_event(owner_id, event_id).await?;
        let guest = self.guest_of(event_id, guest_id).await?;
        self.pdf.render(&event, &guest)
    }

    async fn send_invite(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        guest_id: Uuid,
    ) -> Result<String, DomainError> {
        self.owned_event(owner_id, event_id).await?; // ownership gate
        let guest = self.guest_of(event_id, guest_id).await?;
        let invite_url = format!("{}/invite/{}", self.public_base_url, guest.invite_token);
        self.sender.send(&guest, &invite_url).await?;
        Ok(invite_url)
    }
}
