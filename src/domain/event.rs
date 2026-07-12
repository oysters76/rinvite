use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use uuid::Uuid;

/// A wedding ceremony created and owned by a user. Pure domain type — no
/// framework, transport, or database types leak in here.
#[derive(Debug, Clone)]
pub struct Event {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub bride_name: String,
    pub bride_family_name: String,
    pub groom_name: String,
    pub groom_family_name: String,
    pub event_date: NaiveDate,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    /// The room within the venue, e.g. "The King's Ballroom".
    pub hall_name: String,
    /// The venue as a whole, e.g. "Peradeniya Rest House, Kandy".
    pub venue_name: String,
    /// Guests must RSVP on or before this date.
    pub rsvp_by: NaiveDate,
    pub created_at: DateTime<Utc>,
}

/// The details supplied when creating an event (no id/owner/timestamps yet).
#[derive(Debug, Clone)]
pub struct NewEvent {
    pub bride_name: String,
    pub bride_family_name: String,
    pub groom_name: String,
    pub groom_family_name: String,
    pub event_date: NaiveDate,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub hall_name: String,
    pub venue_name: String,
    pub rsvp_by: NaiveDate,
}

/// Partial update for an event — only the `Some` fields are changed.
#[derive(Debug, Clone, Default)]
pub struct EventUpdate {
    pub bride_name: Option<String>,
    pub bride_family_name: Option<String>,
    pub groom_name: Option<String>,
    pub groom_family_name: Option<String>,
    pub event_date: Option<NaiveDate>,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub hall_name: Option<String>,
    pub venue_name: Option<String>,
    pub rsvp_by: Option<NaiveDate>,
}

impl Event {
    /// Build a new event with a fresh id for the given owner.
    pub fn new(owner_id: Uuid, details: NewEvent, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            owner_id,
            bride_name: details.bride_name,
            bride_family_name: details.bride_family_name,
            groom_name: details.groom_name,
            groom_family_name: details.groom_family_name,
            event_date: details.event_date,
            start_time: details.start_time,
            end_time: details.end_time,
            hall_name: details.hall_name,
            venue_name: details.venue_name,
            rsvp_by: details.rsvp_by,
            created_at: now,
        }
    }

    /// Apply the `Some` fields of a partial update in place.
    pub fn apply_update(&mut self, u: EventUpdate) {
        if let Some(v) = u.bride_name {
            self.bride_name = v;
        }
        if let Some(v) = u.bride_family_name {
            self.bride_family_name = v;
        }
        if let Some(v) = u.groom_name {
            self.groom_name = v;
        }
        if let Some(v) = u.groom_family_name {
            self.groom_family_name = v;
        }
        if let Some(v) = u.event_date {
            self.event_date = v;
        }
        if let Some(v) = u.start_time {
            self.start_time = v;
        }
        if let Some(v) = u.end_time {
            self.end_time = v;
        }
        if let Some(v) = u.hall_name {
            self.hall_name = v;
        }
        if let Some(v) = u.venue_name {
            self.venue_name = v;
        }
        if let Some(v) = u.rsvp_by {
            self.rsvp_by = v;
        }
    }

    /// A `NewEvent` view of the current field values, for re-validation.
    pub fn as_new(&self) -> NewEvent {
        NewEvent {
            bride_name: self.bride_name.clone(),
            bride_family_name: self.bride_family_name.clone(),
            groom_name: self.groom_name.clone(),
            groom_family_name: self.groom_family_name.clone(),
            event_date: self.event_date,
            start_time: self.start_time,
            end_time: self.end_time,
            hall_name: self.hall_name.clone(),
            venue_name: self.venue_name.clone(),
            rsvp_by: self.rsvp_by,
        }
    }
}
