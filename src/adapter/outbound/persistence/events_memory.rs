use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::Event;
use crate::domain::guest::Guest;
use crate::domain::port::outbound::{EventRepository, GuestRepository};

/// In-memory adapter backing both `EventRepository` and `GuestRepository`
/// (events and their guests are one aggregate). Ideal for tests and running
/// without a database.
#[derive(Clone, Default)]
pub struct InMemoryEventStore {
    events: Arc<RwLock<HashMap<Uuid, Event>>>,
    guests: Arc<RwLock<HashMap<Uuid, Guest>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EventRepository for InMemoryEventStore {
    async fn save(&self, event: &Event) -> Result<(), DomainError> {
        self.events.write().await.insert(event.id, event.clone());
        Ok(())
    }

    async fn find(&self, id: Uuid) -> Result<Option<Event>, DomainError> {
        Ok(self.events.read().await.get(&id).cloned())
    }

    async fn list_by_owner(&self, owner_id: Uuid) -> Result<Vec<Event>, DomainError> {
        let mut events: Vec<Event> = self
            .events
            .read()
            .await
            .values()
            .filter(|e| e.owner_id == owner_id)
            .cloned()
            .collect();
        events.sort_by_key(|e| std::cmp::Reverse(e.created_at));
        Ok(events)
    }

    async fn update(&self, event: &Event) -> Result<(), DomainError> {
        self.events.write().await.insert(event.id, event.clone());
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.events.write().await.remove(&id);
        // Mirror the DB's ON DELETE CASCADE: drop this event's guests too.
        self.guests.write().await.retain(|_, g| g.event_id != id);
        Ok(())
    }
}

#[async_trait]
impl GuestRepository for InMemoryEventStore {
    async fn save(&self, guest: &Guest) -> Result<(), DomainError> {
        self.guests.write().await.insert(guest.id, guest.clone());
        Ok(())
    }

    async fn save_many(&self, guests: &[Guest]) -> Result<(), DomainError> {
        // One lock for the whole batch, mirroring the Postgres transaction.
        let mut store = self.guests.write().await;
        for g in guests {
            store.insert(g.id, g.clone());
        }
        Ok(())
    }

    async fn find(&self, id: Uuid) -> Result<Option<Guest>, DomainError> {
        Ok(self.guests.read().await.get(&id).cloned())
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<Guest>, DomainError> {
        Ok(self
            .guests
            .read()
            .await
            .values()
            .find(|g| g.invite_token == token)
            .cloned())
    }

    async fn list_by_event(&self, event_id: Uuid) -> Result<Vec<Guest>, DomainError> {
        let mut guests: Vec<Guest> = self
            .guests
            .read()
            .await
            .values()
            .filter(|g| g.event_id == event_id)
            .cloned()
            .collect();
        guests.sort_by_key(|g| g.created_at);
        Ok(guests)
    }

    async fn update_rsvp(&self, guest: &Guest) -> Result<(), DomainError> {
        self.guests.write().await.insert(guest.id, guest.clone());
        Ok(())
    }

    async fn update(&self, guest: &Guest) -> Result<(), DomainError> {
        self.guests.write().await.insert(guest.id, guest.clone());
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.guests.write().await.remove(&id);
        Ok(())
    }
}
