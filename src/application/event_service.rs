use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::{Event, EventUpdate, NewEvent};
use crate::domain::guest::InviteChannel;
use crate::domain::guest::{Guest, GuestUpdate, NewGuest};
use crate::domain::port::inbound::{BatchSendReport, EventService, SendResult, SendStatus};
use crate::domain::port::outbound::{
    Clock, EventRepository, GuestRepository, InvitePdfRenderer, InviteSender, UserRepository,
};
use crate::domain::validation::{validate_event, validate_guest};

/// Implements the owner-facing `EventService`. Every read/write is scoped to
/// the calling `owner_id`; a resource owned by someone else is reported as
/// `NotFound` so we never confirm its existence.
pub struct EventServiceImpl {
    events: Arc<dyn EventRepository>,
    guests: Arc<dyn GuestRepository>,
    users: Arc<dyn UserRepository>,
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
        users: Arc<dyn UserRepository>,
        pdf: Arc<dyn InvitePdfRenderer>,
        sender: Arc<dyn InviteSender>,
        clock: Arc<dyn Clock>,
        public_base_url: String,
    ) -> Self {
        Self {
            events,
            guests,
            users,
            pdf,
            sender,
            clock,
            public_base_url,
        }
    }

    /// The subscription plan for `owner_id`, or `NotFound` if the user is gone.
    async fn owner_plan(&self, owner_id: Uuid) -> Result<crate::domain::plan::Plan, DomainError> {
        self.users
            .find_by_id(owner_id)
            .await?
            .map(|u| u.plan)
            .ok_or_else(|| DomainError::NotFound("user".to_owned()))
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

        // Plan gate: cap the number of events an owner may create.
        if let Some(max) = self.owner_plan(owner_id).await?.limits().max_events {
            let count = self.events.list_by_owner(owner_id).await?.len() as u32;
            if count >= max {
                return Err(DomainError::LimitReached(format!(
                    "your plan allows {max} event{}",
                    if max == 1 { "" } else { "s" }
                )));
            }
        }

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

    async fn update_event(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        update: EventUpdate,
    ) -> Result<Event, DomainError> {
        let mut event = self.owned_event(owner_id, event_id).await?;
        event.apply_update(update);
        validate_event(&event.as_new())?;
        self.events.update(&event).await?;
        Ok(event)
    }

    async fn delete_event(&self, owner_id: Uuid, event_id: Uuid) -> Result<(), DomainError> {
        self.owned_event(owner_id, event_id).await?; // ownership gate
        self.events.delete(event_id).await
    }

    async fn add_guest(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        details: NewGuest,
    ) -> Result<Guest, DomainError> {
        self.owned_event(owner_id, event_id).await?; // ownership gate
        validate_guest(&details)?;

        // Plan gate: cap the number of guests on a single event.
        if let Some(max) = self
            .owner_plan(owner_id)
            .await?
            .limits()
            .max_guests_per_event
        {
            let count = self.guests.list_by_event(event_id).await?.len() as u32;
            if count >= max {
                return Err(DomainError::LimitReached(format!(
                    "your plan allows {max} guests per event"
                )));
            }
        }

        let guest = Guest::new(event_id, details, self.clock.now());
        self.guests.save(&guest).await?;
        Ok(guest)
    }

    async fn add_guests_bulk(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        details: Vec<NewGuest>,
    ) -> Result<Vec<Guest>, DomainError> {
        self.owned_event(owner_id, event_id).await?; // ownership gate

        // Bound the batch: reject an empty upload, and cap the size so a single
        // request can't be turned into a huge write.
        const MAX_BULK: usize = 500;
        if details.is_empty() {
            return Err(DomainError::InvalidInput(
                "no guests to import".to_owned(),
            ));
        }
        if details.len() > MAX_BULK {
            return Err(DomainError::InvalidInput(format!(
                "cannot import more than {MAX_BULK} guests at once"
            )));
        }

        // Validate every row up front — nothing is inserted unless all pass.
        for (i, d) in details.iter().enumerate() {
            validate_guest(d).map_err(|e| match e {
                DomainError::InvalidInput(msg) => {
                    DomainError::InvalidInput(format!("row {}: {msg}", i + 1))
                }
                other => other,
            })?;
        }

        // Plan gate: the whole batch must fit under the per-event guest cap.
        if let Some(max) = self
            .owner_plan(owner_id)
            .await?
            .limits()
            .max_guests_per_event
        {
            let existing = self.guests.list_by_event(event_id).await?.len() as u32;
            if existing + details.len() as u32 > max {
                return Err(DomainError::LimitReached(format!(
                    "your plan allows {max} guests per event"
                )));
            }
        }

        let now = self.clock.now();
        let guests: Vec<Guest> = details
            .into_iter()
            .map(|d| Guest::new(event_id, d, now))
            .collect();
        self.guests.save_many(&guests).await?;
        Ok(guests)
    }

    async fn list_guests(&self, owner_id: Uuid, event_id: Uuid) -> Result<Vec<Guest>, DomainError> {
        self.owned_event(owner_id, event_id).await?; // ownership gate
        self.guests.list_by_event(event_id).await
    }

    async fn get_guest(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        guest_id: Uuid,
    ) -> Result<Guest, DomainError> {
        self.owned_event(owner_id, event_id).await?; // ownership gate
        self.guest_of(event_id, guest_id).await
    }

    async fn update_guest(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        guest_id: Uuid,
        update: GuestUpdate,
    ) -> Result<Guest, DomainError> {
        self.owned_event(owner_id, event_id).await?; // ownership gate
        let mut guest = self.guest_of(event_id, guest_id).await?;
        guest.apply_update(update);
        validate_guest(&guest.as_new())?;
        self.guests.update(&guest).await?;
        Ok(guest)
    }

    async fn delete_guest(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
        guest_id: Uuid,
    ) -> Result<(), DomainError> {
        self.owned_event(owner_id, event_id).await?; // ownership gate
        self.guest_of(event_id, guest_id).await?; // exists + belongs to event
        self.guests.delete(guest_id).await
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
        let event = self.owned_event(owner_id, event_id).await?; // ownership gate
        let guest = self.guest_of(event_id, guest_id).await?;
        let invite_url = format!("{}/i/{}", self.public_base_url, guest.invite_token);
        self.sender.send(&event, &guest, &invite_url).await?;
        Ok(invite_url)
    }

    async fn render_print_batch(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
    ) -> Result<Vec<u8>, DomainError> {
        let event = self.owned_event(owner_id, event_id).await?;
        let print_guests: Vec<Guest> = self
            .guests
            .list_by_event(event_id)
            .await?
            .into_iter()
            .filter(|g| g.channel == InviteChannel::Print)
            .collect();
        if print_guests.is_empty() {
            return Err(DomainError::InvalidInput(
                "event has no print guests to render".to_owned(),
            ));
        }
        self.pdf.render_all(&event, &print_guests)
    }

    async fn send_einvite_batch(
        &self,
        owner_id: Uuid,
        event_id: Uuid,
    ) -> Result<BatchSendReport, DomainError> {
        let event = self.owned_event(owner_id, event_id).await?; // ownership gate
        let einvite_guests: Vec<Guest> = self
            .guests
            .list_by_event(event_id)
            .await?
            .into_iter()
            .filter(|g| g.channel == InviteChannel::EInvite)
            .collect();

        let mut results = Vec::with_capacity(einvite_guests.len());
        let (mut sent, mut failed) = (0usize, 0usize);
        // Sequential: one guest at a time, and a single failure never aborts
        // the batch.
        for guest in &einvite_guests {
            let url = format!("{}/i/{}", self.public_base_url, guest.invite_token);
            let (status, detail) = match self.sender.send(&event, guest, &url).await {
                Ok(()) => {
                    sent += 1;
                    (SendStatus::Sent, None)
                }
                Err(e) => {
                    failed += 1;
                    (SendStatus::Failed, Some(e.to_string()))
                }
            };
            results.push(SendResult {
                guest_id: guest.id,
                guest_name: guest.name.clone(),
                status,
                detail,
            });
        }

        Ok(BatchSendReport {
            total: einvite_guests.len(),
            sent,
            failed,
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};

    use super::*;
    use crate::adapter::outbound::persistence::events_memory::InMemoryEventStore;
    use crate::adapter::outbound::persistence::memory::InMemoryUserRepository;
    use crate::domain::model::User;
    use crate::domain::plan::Plan;

    /// A user repository holding a single owner on the given plan (email already
    /// verified), returning the repo and that owner's id.
    async fn user_on_plan(plan: Plan) -> (Arc<InMemoryUserRepository>, Uuid) {
        let users = Arc::new(InMemoryUserRepository::new());
        let now = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let mut u = User::new("owner@example.com".into(), "hash".into(), now);
        u.plan = plan;
        u.email_verified = true;
        users.save(&u).await.unwrap();
        (users, u.id)
    }

    /// Records how many guests the last `render_all` batch received.
    struct FakePdf {
        last_batch: Mutex<usize>,
    }
    impl InvitePdfRenderer for FakePdf {
        fn render(&self, _e: &Event, _g: &Guest) -> Result<Vec<u8>, DomainError> {
            Ok(vec![1])
        }
        fn render_all(&self, _e: &Event, guests: &[Guest]) -> Result<Vec<u8>, DomainError> {
            *self.last_batch.lock().unwrap() = guests.len();
            Ok(vec![1; guests.len()])
        }
    }

    /// Fails delivery for any guest named "boom".
    struct FlakySender;
    #[async_trait]
    impl InviteSender for FlakySender {
        async fn send(&self, _event: &Event, guest: &Guest, _url: &str) -> Result<(), DomainError> {
            if guest.name == "boom" {
                Err(DomainError::Repository("smtp down".to_owned()))
            } else {
                Ok(())
            }
        }
    }

    struct FixedClock(DateTime<Utc>);
    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn sample_event() -> NewEvent {
        NewEvent {
            bride_name: "A".into(),
            bride_family_name: "B".into(),
            groom_name: "C".into(),
            groom_family_name: "D".into(),
            event_date: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
            hall_name: "H".into(),
            venue_name: "V".into(),
            rsvp_by: NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
            poruwa_ceremony_time: None,
        }
    }

    fn guest(name: &str, channel: InviteChannel) -> NewGuest {
        NewGuest {
            name: name.into(),
            channel,
            email: None,
            phone: None,
            max_party_size: 2,
        }
    }

    #[tokio::test]
    async fn bulk_send_reports_per_guest_and_print_filters_channel() {
        let store = Arc::new(InMemoryEventStore::new());
        let pdf = Arc::new(FakePdf {
            last_batch: Mutex::new(0),
        });
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(
            Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        ));
        // Max plan: no event/guest caps so this batching test is unaffected.
        let (users, owner) = user_on_plan(Plan::Max).await;
        let svc = EventServiceImpl::new(
            store.clone(),
            store.clone(),
            users,
            pdf.clone(),
            Arc::new(FlakySender),
            clock,
            "http://x".into(),
        );

        let ev = svc.create_event(owner, sample_event()).await.unwrap();
        svc.add_guest(owner, ev.id, guest("ok", InviteChannel::EInvite))
            .await
            .unwrap();
        svc.add_guest(owner, ev.id, guest("boom", InviteChannel::EInvite))
            .await
            .unwrap();
        svc.add_guest(owner, ev.id, guest("printy", InviteChannel::Print))
            .await
            .unwrap();

        // Bulk send touches only the two e-invite guests; one fails.
        let report = svc.send_einvite_batch(owner, ev.id).await.unwrap();
        assert_eq!(report.total, 2);
        assert_eq!(report.sent, 1);
        assert_eq!(report.failed, 1);
        assert!(
            report
                .results
                .iter()
                .any(|r| r.guest_name == "boom" && r.status == SendStatus::Failed)
        );

        // Bulk print renders only the single print-channel guest.
        svc.render_print_batch(owner, ev.id).await.unwrap();
        assert_eq!(*pdf.last_batch.lock().unwrap(), 1);

        // No print guests for a different (empty) event -> error.
        let ev2 = svc.create_event(owner, sample_event()).await.unwrap();
        assert!(svc.render_print_batch(owner, ev2.id).await.is_err());

        // Ownership: a different user cannot bulk-send someone else's event.
        assert!(matches!(
            svc.send_einvite_batch(Uuid::new_v4(), ev.id).await,
            Err(DomainError::NotFound(_))
        ));
    }

    /// Build a service on the `Max` plan (no caps) plus its owner id.
    async fn make_svc() -> (EventServiceImpl, std::sync::Arc<InMemoryEventStore>, Uuid) {
        let store = Arc::new(InMemoryEventStore::new());
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(
            Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        ));
        let (users, owner) = user_on_plan(Plan::Max).await;
        let svc = EventServiceImpl::new(
            store.clone(),
            store.clone(),
            users,
            Arc::new(FakePdf {
                last_batch: Mutex::new(0),
            }),
            Arc::new(FlakySender),
            clock,
            "http://x".into(),
        );
        (svc, store, owner)
    }

    #[tokio::test]
    async fn partial_update_changes_only_sent_fields_and_revalidates() {
        let (svc, _, owner) = make_svc().await;
        let ev = svc.create_event(owner, sample_event()).await.unwrap();

        // Change just the venue; everything else is untouched.
        let updated = svc
            .update_event(
                owner,
                ev.id,
                EventUpdate {
                    venue_name: Some("New Hall".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.venue_name, "New Hall");
        assert_eq!(updated.bride_name, "A"); // unchanged

        // An update that violates an invariant is rejected.
        assert!(matches!(
            svc.update_event(
                owner,
                ev.id,
                EventUpdate {
                    end_time: Some(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
                    ..Default::default()
                },
            )
            .await,
            Err(DomainError::InvalidInput(_))
        ));

        // Foreign owner can't touch it.
        assert!(matches!(
            svc.update_event(Uuid::new_v4(), ev.id, EventUpdate::default())
                .await,
            Err(DomainError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn guest_update_delete_and_event_delete_cascades() {
        let (svc, _, owner) = make_svc().await;
        let ev = svc.create_event(owner, sample_event()).await.unwrap();
        let g = svc
            .add_guest(owner, ev.id, guest("Ravi", InviteChannel::EInvite))
            .await
            .unwrap();

        // Update guest contact + max party.
        let updated = svc
            .update_guest(
                owner,
                ev.id,
                g.id,
                GuestUpdate {
                    email: Some(Some("ravi@ex.com".into())),
                    max_party_size: Some(4),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.email.as_deref(), Some("ravi@ex.com"));
        assert_eq!(updated.max_party_size, 4);

        // Delete guest.
        svc.delete_guest(owner, ev.id, g.id).await.unwrap();
        assert!(svc.list_guests(owner, ev.id).await.unwrap().is_empty());

        // Deleting the event cascades: add a guest, delete event, event gone.
        svc.add_guest(owner, ev.id, guest("X", InviteChannel::Print))
            .await
            .unwrap();
        svc.delete_event(owner, ev.id).await.unwrap();
        assert!(matches!(
            svc.get_event(owner, ev.id).await,
            Err(DomainError::NotFound(_))
        ));
        // The cascaded guest is gone too (list on a deleted event -> NotFound).
        assert!(matches!(
            svc.list_guests(owner, ev.id).await,
            Err(DomainError::NotFound(_))
        ));
    }

    /// Build a service whose single owner is on `plan`; returns svc + owner id.
    async fn svc_on_plan(plan: Plan) -> (EventServiceImpl, Uuid) {
        let store = Arc::new(InMemoryEventStore::new());
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(
            Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        ));
        let (users, owner) = user_on_plan(plan).await;
        let svc = EventServiceImpl::new(
            store.clone(),
            store.clone(),
            users,
            Arc::new(FakePdf {
                last_batch: Mutex::new(0),
            }),
            Arc::new(FlakySender),
            clock,
            "http://x".into(),
        );
        (svc, owner)
    }

    #[tokio::test]
    async fn free_plan_caps_events_at_one() {
        let (svc, owner) = svc_on_plan(Plan::Free).await;
        svc.create_event(owner, sample_event()).await.unwrap();
        // Second event is over the free cap.
        assert!(matches!(
            svc.create_event(owner, sample_event()).await,
            Err(DomainError::LimitReached(_))
        ));
    }

    #[tokio::test]
    async fn free_plan_caps_guests_at_ten_per_event() {
        let (svc, owner) = svc_on_plan(Plan::Free).await;
        let ev = svc.create_event(owner, sample_event()).await.unwrap();
        for i in 0..10 {
            svc.add_guest(owner, ev.id, guest(&format!("g{i}"), InviteChannel::Print))
                .await
                .unwrap();
        }
        // The 11th guest is over the free cap.
        assert!(matches!(
            svc.add_guest(owner, ev.id, guest("g11", InviteChannel::Print))
                .await,
            Err(DomainError::LimitReached(_))
        ));
    }

    #[tokio::test]
    async fn max_plan_is_unlimited() {
        let (svc, owner) = svc_on_plan(Plan::Max).await;
        // Many events, each with more guests than any finite cap allows.
        for _ in 0..3 {
            let ev = svc.create_event(owner, sample_event()).await.unwrap();
            for i in 0..150 {
                svc.add_guest(owner, ev.id, guest(&format!("g{i}"), InviteChannel::Print))
                    .await
                    .unwrap();
            }
        }
    }

    #[tokio::test]
    async fn bulk_add_inserts_every_valid_guest() {
        let (svc, owner) = svc_on_plan(Plan::Max).await;
        let ev = svc.create_event(owner, sample_event()).await.unwrap();
        let batch = vec![
            guest("Ann", InviteChannel::EInvite),
            guest("Bo", InviteChannel::Print),
            guest("Cy", InviteChannel::EInvite),
        ];
        let added = svc.add_guests_bulk(owner, ev.id, batch).await.unwrap();
        assert_eq!(added.len(), 3);
        assert_eq!(svc.list_guests(owner, ev.id).await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn bulk_add_is_all_or_nothing_on_an_invalid_row() {
        let (svc, owner) = svc_on_plan(Plan::Max).await;
        let ev = svc.create_event(owner, sample_event()).await.unwrap();
        // Second row has an empty name — the whole batch must be rejected.
        let batch = vec![
            guest("Ann", InviteChannel::EInvite),
            guest("   ", InviteChannel::Print),
            guest("Cy", InviteChannel::EInvite),
        ];
        assert!(matches!(
            svc.add_guests_bulk(owner, ev.id, batch).await,
            Err(DomainError::InvalidInput(_))
        ));
        // Nothing was persisted.
        assert!(svc.list_guests(owner, ev.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn bulk_add_rejects_batch_that_would_exceed_plan_cap() {
        let (svc, owner) = svc_on_plan(Plan::Free).await; // 10 guests / event
        let ev = svc.create_event(owner, sample_event()).await.unwrap();
        let batch: Vec<NewGuest> = (0..11)
            .map(|i| guest(&format!("g{i}"), InviteChannel::Print))
            .collect();
        assert!(matches!(
            svc.add_guests_bulk(owner, ev.id, batch).await,
            Err(DomainError::LimitReached(_))
        ));
        // Atomic: no partial insert up to the cap.
        assert!(svc.list_guests(owner, ev.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn bulk_add_rejects_an_empty_batch() {
        let (svc, owner) = svc_on_plan(Plan::Max).await;
        let ev = svc.create_event(owner, sample_event()).await.unwrap();
        assert!(matches!(
            svc.add_guests_bulk(owner, ev.id, vec![]).await,
            Err(DomainError::InvalidInput(_))
        ));
    }
}
