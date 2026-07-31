use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use uuid::Uuid;

/// Which side of the couple is listed first everywhere the two appear together
/// on the invite — family names, personal names, RSVP phone numbers, monogram,
/// footer, and message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Precedence {
    /// Bride's side first — the historical default.
    #[default]
    Bride,
    /// Groom's side first.
    Groom,
}

impl Precedence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Precedence::Bride => "bride",
            Precedence::Groom => "groom",
        }
    }

    /// Lenient parse: anything other than `"groom"` (including blank input and
    /// legacy rows) resolves to the `Bride` default, keeping current invites
    /// bride-first.
    pub fn parse(s: &str) -> Self {
        match s {
            "groom" => Precedence::Groom,
            _ => Precedence::Bride,
        }
    }
}

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
    /// RSVP contact number for the bride's side, e.g. "+94 71 195 4412".
    pub bride_phone: String,
    /// RSVP contact number for the groom's side.
    pub groom_phone: String,
    /// Whose side is listed first on the invite.
    pub precedence: Precedence,
    pub event_date: NaiveDate,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    /// The room within the venue, e.g. "The King's Ballroom".
    pub hall_name: String,
    /// The venue as a whole, e.g. "Peradeniya Rest House, Kandy".
    pub venue_name: String,
    /// Guests must RSVP on or before this date.
    pub rsvp_by: NaiveDate,
    /// Optional auspicious time for the Poruwa ceremony.
    pub poruwa_ceremony_time: Option<NaiveTime>,
    pub created_at: DateTime<Utc>,
}

/// The details supplied when creating an event (no id/owner/timestamps yet).
#[derive(Debug, Clone)]
pub struct NewEvent {
    pub bride_name: String,
    pub bride_family_name: String,
    pub groom_name: String,
    pub groom_family_name: String,
    pub bride_phone: String,
    pub groom_phone: String,
    pub precedence: Precedence,
    pub event_date: NaiveDate,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub hall_name: String,
    pub venue_name: String,
    pub rsvp_by: NaiveDate,
    pub poruwa_ceremony_time: Option<NaiveTime>,
}

/// Partial update for an event — only the `Some` fields are changed.
#[derive(Debug, Clone, Default)]
pub struct EventUpdate {
    pub bride_name: Option<String>,
    pub bride_family_name: Option<String>,
    pub groom_name: Option<String>,
    pub groom_family_name: Option<String>,
    pub bride_phone: Option<String>,
    pub groom_phone: Option<String>,
    pub precedence: Option<Precedence>,
    pub event_date: Option<NaiveDate>,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub hall_name: Option<String>,
    pub venue_name: Option<String>,
    pub rsvp_by: Option<NaiveDate>,
    /// `Some(inner)` sets the value (`inner` may be `None` to clear); `None`
    /// leaves it unchanged — mirrors the guest-contact update idiom.
    pub poruwa_ceremony_time: Option<Option<NaiveTime>>,
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
            bride_phone: details.bride_phone,
            groom_phone: details.groom_phone,
            precedence: details.precedence,
            event_date: details.event_date,
            start_time: details.start_time,
            end_time: details.end_time,
            hall_name: details.hall_name,
            venue_name: details.venue_name,
            rsvp_by: details.rsvp_by,
            poruwa_ceremony_time: details.poruwa_ceremony_time,
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
        if let Some(v) = u.bride_phone {
            self.bride_phone = v;
        }
        if let Some(v) = u.groom_phone {
            self.groom_phone = v;
        }
        if let Some(v) = u.precedence {
            self.precedence = v;
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
        if let Some(v) = u.poruwa_ceremony_time {
            self.poruwa_ceremony_time = v;
        }
    }

    /// A `NewEvent` view of the current field values, for re-validation.
    pub fn as_new(&self) -> NewEvent {
        NewEvent {
            bride_name: self.bride_name.clone(),
            bride_family_name: self.bride_family_name.clone(),
            groom_name: self.groom_name.clone(),
            groom_family_name: self.groom_family_name.clone(),
            bride_phone: self.bride_phone.clone(),
            groom_phone: self.groom_phone.clone(),
            precedence: self.precedence,
            event_date: self.event_date,
            start_time: self.start_time,
            end_time: self.end_time,
            hall_name: self.hall_name.clone(),
            venue_name: self.venue_name.clone(),
            rsvp_by: self.rsvp_by,
            poruwa_ceremony_time: self.poruwa_ceremony_time,
        }
    }

    /// Bride/groom personal names as `(first, second)`, ordered by precedence.
    pub fn ordered_names(&self) -> (&str, &str) {
        self.order((&self.bride_name, &self.groom_name))
    }

    /// Bride/groom family names as `(first, second)`, ordered by precedence.
    pub fn ordered_family_names(&self) -> (&str, &str) {
        self.order((&self.bride_family_name, &self.groom_family_name))
    }

    /// Bride/groom RSVP phone numbers as `(first, second)`, ordered by precedence.
    pub fn ordered_phones(&self) -> (&str, &str) {
        self.order((&self.bride_phone, &self.groom_phone))
    }

    /// Return a `(bride_side, groom_side)` pair as `(first, second)`: unchanged
    /// when the bride takes precedence, swapped when the groom does.
    fn order<'a>(&self, sides: (&'a str, &'a str)) -> (&'a str, &'a str) {
        match self.precedence {
            Precedence::Bride => sides,
            Precedence::Groom => (sides.1, sides.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn event_with(precedence: Precedence) -> Event {
        Event::new(
            Uuid::new_v4(),
            NewEvent {
                bride_name: "Bride".into(),
                bride_family_name: "BrideFam".into(),
                groom_name: "Groom".into(),
                groom_family_name: "GroomFam".into(),
                bride_phone: "111".into(),
                groom_phone: "222".into(),
                precedence,
                event_date: NaiveDate::from_ymd_opt(2026, 9, 25).unwrap(),
                start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                hall_name: "Hall".into(),
                venue_name: "Venue".into(),
                rsvp_by: NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
                poruwa_ceremony_time: None,
            },
            Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        )
    }

    #[test]
    fn bride_precedence_keeps_bride_first() {
        let e = event_with(Precedence::Bride);
        assert_eq!(e.ordered_names(), ("Bride", "Groom"));
        assert_eq!(e.ordered_family_names(), ("BrideFam", "GroomFam"));
        assert_eq!(e.ordered_phones(), ("111", "222"));
    }

    #[test]
    fn groom_precedence_swaps_to_groom_first() {
        let e = event_with(Precedence::Groom);
        assert_eq!(e.ordered_names(), ("Groom", "Bride"));
        assert_eq!(e.ordered_family_names(), ("GroomFam", "BrideFam"));
        assert_eq!(e.ordered_phones(), ("222", "111"));
    }

    #[test]
    fn precedence_parse_defaults_to_bride() {
        assert_eq!(Precedence::parse("groom"), Precedence::Groom);
        assert_eq!(Precedence::parse("bride"), Precedence::Bride);
        assert_eq!(Precedence::parse(""), Precedence::Bride);
        assert_eq!(Precedence::parse("nonsense"), Precedence::Bride);
    }
}
