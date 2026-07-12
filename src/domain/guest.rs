use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::error::DomainError;

/// How a guest is invited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteChannel {
    /// A printed, hand-delivered card — we generate a PDF.
    Print,
    /// A hosted web page the guest opens to RSVP.
    EInvite,
}

impl InviteChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            InviteChannel::Print => "print",
            InviteChannel::EInvite => "einvite",
        }
    }

    pub fn parse(s: &str) -> Result<Self, DomainError> {
        match s {
            "print" => Ok(InviteChannel::Print),
            "einvite" => Ok(InviteChannel::EInvite),
            other => Err(DomainError::Repository(format!(
                "unknown invite channel: {other}"
            ))),
        }
    }
}

/// Where a guest stands on attending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsvpStatus {
    Pending,
    Attending,
    Declined,
}

impl RsvpStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RsvpStatus::Pending => "pending",
            RsvpStatus::Attending => "attending",
            RsvpStatus::Declined => "declined",
        }
    }

    pub fn parse(s: &str) -> Result<Self, DomainError> {
        match s {
            "pending" => Ok(RsvpStatus::Pending),
            "attending" => Ok(RsvpStatus::Attending),
            "declined" => Ok(RsvpStatus::Declined),
            other => Err(DomainError::Repository(format!(
                "unknown rsvp status: {other}"
            ))),
        }
    }
}

/// A single invitee on an event's guest list.
#[derive(Debug, Clone)]
pub struct Guest {
    pub id: Uuid,
    pub event_id: Uuid,
    pub name: String,
    pub channel: InviteChannel,
    /// Optional contact details — where a real sender delivers the e-invite.
    pub email: Option<String>,
    pub phone: Option<String>,
    /// Maximum total headcount for this guest's party, *including* the guest
    /// themselves (e.g. 2 = the guest plus one companion).
    pub max_party_size: u16,
    /// Unguessable token used as the e-invite URL path and RSVP capability.
    pub invite_token: String,
    pub rsvp_status: RsvpStatus,
    /// Actual headcount the guest committed to, set once they respond.
    pub party_size: Option<u16>,
    pub responded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Details supplied when adding a guest to an event.
#[derive(Debug, Clone)]
pub struct NewGuest {
    pub name: String,
    pub channel: InviteChannel,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub max_party_size: u16,
}

/// Partial update for a guest — only the `Some` fields are changed. Contact
/// fields use `Option<Option<String>>`: outer `None` = leave unchanged, inner
/// `None` = clear it.
#[derive(Debug, Clone, Default)]
pub struct GuestUpdate {
    pub name: Option<String>,
    pub channel: Option<InviteChannel>,
    pub email: Option<Option<String>>,
    pub phone: Option<Option<String>>,
    pub max_party_size: Option<u16>,
}

impl Guest {
    /// Build a new guest with a fresh id and invite token, RSVP still pending.
    pub fn new(event_id: Uuid, details: NewGuest, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_id,
            name: details.name,
            channel: details.channel,
            email: details.email,
            phone: details.phone,
            max_party_size: details.max_party_size,
            // Two v4 UUIDs give ~244 bits of entropy — plenty to keep the
            // invite link unguessable.
            invite_token: format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()),
            rsvp_status: RsvpStatus::Pending,
            party_size: None,
            responded_at: None,
            created_at: now,
        }
    }

    /// Apply the `Some` fields of a partial update in place.
    pub fn apply_update(&mut self, u: GuestUpdate) {
        if let Some(v) = u.name {
            self.name = v;
        }
        if let Some(v) = u.channel {
            self.channel = v;
        }
        if let Some(v) = u.email {
            self.email = v;
        }
        if let Some(v) = u.phone {
            self.phone = v;
        }
        if let Some(v) = u.max_party_size {
            self.max_party_size = v;
        }
    }

    /// A `NewGuest` view of the current field values, for re-validation.
    pub fn as_new(&self) -> NewGuest {
        NewGuest {
            name: self.name.clone(),
            channel: self.channel,
            email: self.email.clone(),
            phone: self.phone.clone(),
            max_party_size: self.max_party_size,
        }
    }

    /// Apply a guest's RSVP. This is the one place the RSVP rules live:
    /// the deadline must not have passed, and an attending party must be
    /// between 1 and `max_party_size` (headcount includes the guest).
    pub fn respond(
        &mut self,
        attending: bool,
        party_size: u16,
        now: DateTime<Utc>,
        rsvp_by: NaiveDate,
    ) -> Result<(), DomainError> {
        // Deadline is inclusive of the whole rsvp_by day.
        if now.date_naive() > rsvp_by {
            return Err(DomainError::RsvpClosed);
        }

        if attending {
            if party_size < 1 || party_size > self.max_party_size {
                return Err(DomainError::PartySizeExceeded);
            }
            self.rsvp_status = RsvpStatus::Attending;
            self.party_size = Some(party_size);
        } else {
            self.rsvp_status = RsvpStatus::Declined;
            self.party_size = None;
        }
        self.responded_at = Some(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn guest(max: u16) -> Guest {
        let now = Utc.with_ymd_and_hms(2026, 7, 11, 12, 0, 0).unwrap();
        Guest::new(
            Uuid::new_v4(),
            NewGuest {
                name: "Ravi".to_owned(),
                channel: InviteChannel::EInvite,
                email: None,
                phone: None,
                max_party_size: max,
            },
            now,
        )
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 11, 12, 0, 0).unwrap()
    }
    fn deadline() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 12, 1).unwrap()
    }

    #[test]
    fn attending_within_max_is_recorded() {
        let mut g = guest(2);
        g.respond(true, 2, now(), deadline()).unwrap();
        assert_eq!(g.rsvp_status, RsvpStatus::Attending);
        assert_eq!(g.party_size, Some(2));
        assert!(g.responded_at.is_some());
    }

    #[test]
    fn party_over_max_or_zero_is_rejected() {
        let mut g = guest(2);
        assert!(matches!(
            g.respond(true, 3, now(), deadline()),
            Err(DomainError::PartySizeExceeded)
        ));
        assert!(matches!(
            g.respond(true, 0, now(), deadline()),
            Err(DomainError::PartySizeExceeded)
        ));
        // A rejected response leaves the guest untouched.
        assert_eq!(g.rsvp_status, RsvpStatus::Pending);
    }

    #[test]
    fn declining_clears_party_size() {
        let mut g = guest(2);
        g.respond(true, 2, now(), deadline()).unwrap();
        g.respond(false, 0, now(), deadline()).unwrap();
        assert_eq!(g.rsvp_status, RsvpStatus::Declined);
        assert_eq!(g.party_size, None);
    }

    #[test]
    fn past_the_deadline_is_closed() {
        let mut g = guest(2);
        let after = Utc.with_ymd_and_hms(2026, 12, 2, 9, 0, 0).unwrap();
        assert!(matches!(
            g.respond(true, 1, after, deadline()),
            Err(DomainError::RsvpClosed)
        ));
    }
}
