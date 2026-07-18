use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::error::DomainError;
use crate::domain::event::Event;
use crate::domain::guest::{Guest, InviteChannel};
use crate::domain::port::outbound::{EmailClient, InviteSender, SmsClient, WhatsAppClient};

use super::message::MessageTemplates;

/// Routes an e-invite to a real channel. Prefers WhatsApp when the guest has a
/// phone, otherwise email; a guest with neither is an error (surfaced as a
/// failed row in the send report). Non-e-invite guests are skipped (they get a
/// printed card, not a message). The SMS client is retained but currently
/// unused — phone delivery goes over WhatsApp.
pub struct DispatchSender {
    email: Arc<dyn EmailClient>,
    whatsapp: Arc<dyn WhatsAppClient>,
    /// Retained so SMS delivery can be re-enabled without re-wiring; phone
    /// delivery currently routes through `whatsapp`.
    #[allow(dead_code)]
    sms: Arc<dyn SmsClient>,
    templates: MessageTemplates,
}

impl DispatchSender {
    pub fn new(
        email: Arc<dyn EmailClient>,
        whatsapp: Arc<dyn WhatsAppClient>,
        sms: Arc<dyn SmsClient>,
        templates: MessageTemplates,
    ) -> Self {
        Self {
            email,
            whatsapp,
            sms,
            templates,
        }
    }
}

fn non_empty(s: &Option<String>) -> Option<&str> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

#[async_trait]
impl InviteSender for DispatchSender {
    async fn send(
        &self,
        event: &Event,
        guest: &Guest,
        invite_url: &str,
    ) -> Result<(), DomainError> {
        if guest.channel != InviteChannel::EInvite {
            return Ok(());
        }

        if let Some(phone) = non_empty(&guest.phone) {
            let body = self.templates.render_whatsapp(event, guest, invite_url);
            return self.whatsapp.send_whatsapp(phone, &body).await;
        }

        if let Some(email) = non_empty(&guest.email) {
            let m = self.templates.render_email(event, guest, invite_url);
            return self
                .email
                .send_email(email, &m.subject, &m.html, &m.text)
                .await;
        }

        Err(DomainError::InvalidInput(format!(
            "guest \"{}\" has no phone or email on file",
            guest.name
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::domain::event::{Event, NewEvent};
    use crate::domain::guest::{Guest, NewGuest};
    use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};

    #[derive(Default)]
    struct Spy {
        emails: Mutex<Vec<String>>,
        whatsapps: Mutex<Vec<String>>,
        smses: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl EmailClient for Spy {
        async fn send_email(
            &self,
            to: &str,
            _s: &str,
            _h: &str,
            _t: &str,
        ) -> Result<(), DomainError> {
            self.emails.lock().unwrap().push(to.to_owned());
            Ok(())
        }
    }

    #[async_trait]
    impl WhatsAppClient for Spy {
        async fn send_whatsapp(&self, to: &str, _b: &str) -> Result<(), DomainError> {
            self.whatsapps.lock().unwrap().push(to.to_owned());
            Ok(())
        }
    }

    #[async_trait]
    impl SmsClient for Spy {
        async fn send_sms(&self, to: &str, _b: &str) -> Result<(), DomainError> {
            self.smses.lock().unwrap().push(to.to_owned());
            Ok(())
        }
    }

    fn event() -> Event {
        let now = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        Event::new(
            uuid::Uuid::new_v4(),
            NewEvent {
                bride_name: "A".into(),
                bride_family_name: "X".into(),
                groom_name: "B".into(),
                groom_family_name: "Y".into(),
                event_date: NaiveDate::from_ymd_opt(2026, 9, 25).unwrap(),
                start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                hall_name: "Hall".into(),
                venue_name: "Venue".into(),
                rsvp_by: NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
                poruwa_ceremony_time: None,
            },
            now,
        )
    }

    fn guest(event_id: uuid::Uuid, email: Option<&str>, phone: Option<&str>) -> Guest {
        let now = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        Guest::new(
            event_id,
            NewGuest {
                name: "Guest".into(),
                channel: InviteChannel::EInvite,
                email: email.map(str::to_owned),
                phone: phone.map(str::to_owned),
                max_party_size: 2,
            },
            now,
        )
    }

    fn sender(spy: Arc<Spy>) -> DispatchSender {
        DispatchSender::new(
            spy.clone(),
            spy.clone(),
            spy,
            MessageTemplates::from_env().expect("templates load"),
        )
    }

    #[tokio::test]
    async fn phone_routes_to_whatsapp_even_with_email() {
        let spy = Arc::new(Spy::default());
        let e = event();
        let g = guest(e.id, Some("g@x.com"), Some("+94771234567"));
        sender(spy.clone()).send(&e, &g, "u").await.unwrap();
        assert_eq!(spy.whatsapps.lock().unwrap().len(), 1);
        assert_eq!(spy.smses.lock().unwrap().len(), 0);
        assert_eq!(spy.emails.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn email_only_routes_to_email() {
        let spy = Arc::new(Spy::default());
        let e = event();
        let g = guest(e.id, Some("g@x.com"), None);
        sender(spy.clone()).send(&e, &g, "u").await.unwrap();
        assert_eq!(spy.emails.lock().unwrap().len(), 1);
        assert_eq!(spy.whatsapps.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn no_contact_errors() {
        let spy = Arc::new(Spy::default());
        let e = event();
        let g = guest(e.id, None, Some("   "));
        let err = sender(spy).send(&e, &g, "u").await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidInput(_)));
    }
}
