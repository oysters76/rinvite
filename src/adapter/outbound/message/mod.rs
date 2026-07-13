use chrono::{Datelike, NaiveTime, Timelike};

use crate::domain::error::DomainError;
use crate::domain::event::Event;
use crate::domain::guest::Guest;

// Editable message templates, embedded at compile time and overridable via env.
const DEFAULT_EMAIL_HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/messages/email.html"
));
const DEFAULT_EMAIL_TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/messages/email.txt"
));
const DEFAULT_EMAIL_SUBJECT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/messages/email-subject.txt"
));
const DEFAULT_WHATSAPP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/messages/whatsapp.txt"
));

pub struct RenderedEmail {
    pub subject: String,
    pub html: String,
    pub text: String,
}

/// Loads the message templates (embedded default or the env-pointed file) and
/// renders them for a given event/guest/invite link.
pub struct MessageTemplates {
    email_html: String,
    email_text: String,
    email_subject: String,
    whatsapp: String,
}

impl MessageTemplates {
    pub fn from_env() -> Result<Self, DomainError> {
        Ok(Self {
            email_html: load("EMAIL_TEMPLATE_HTML", DEFAULT_EMAIL_HTML)?,
            email_text: load("EMAIL_TEMPLATE_TEXT", DEFAULT_EMAIL_TEXT)?,
            email_subject: load("EMAIL_SUBJECT", DEFAULT_EMAIL_SUBJECT)?,
            whatsapp: load("WHATSAPP_TEMPLATE", DEFAULT_WHATSAPP)?,
        })
    }

    pub fn render_email(&self, event: &Event, guest: &Guest, invite_url: &str) -> RenderedEmail {
        let vars = build_vars(event, guest, invite_url);
        RenderedEmail {
            subject: fill(&self.email_subject, &vars, false).trim().to_owned(),
            html: fill(&self.email_html, &vars, true),
            text: fill(&self.email_text, &vars, false),
        }
    }

    pub fn render_whatsapp(&self, event: &Event, guest: &Guest, invite_url: &str) -> String {
        fill(&self.whatsapp, &build_vars(event, guest, invite_url), false)
    }
}

fn load(env_var: &str, default: &str) -> Result<String, DomainError> {
    match std::env::var(env_var) {
        Ok(path) => std::fs::read_to_string(&path)
            .map_err(|e| DomainError::Repository(format!("cannot read {env_var} {path}: {e}"))),
        Err(_) => Ok(default.to_owned()),
    }
}

fn build_vars(e: &Event, g: &Guest, invite_url: &str) -> Vec<(&'static str, String)> {
    let couple = format!("{} & {}", e.bride_name, e.groom_name);
    vec![
        ("guest_name", g.name.clone()),
        ("couple", couple),
        ("bride_name", e.bride_name.clone()),
        ("groom_name", e.groom_name.clone()),
        ("date", e.event_date.format("%A, %e %B %Y").to_string()),
        (
            "time",
            format!("{} to {}", fmt_time(e.start_time), fmt_time(e.end_time)),
        ),
        ("venue", e.venue_name.clone()),
        ("hall", e.hall_name.clone()),
        (
            "rsvp_by",
            format!("{} {}", ordinal(e.rsvp_by.day()), e.rsvp_by.format("%B")),
        ),
        ("invite_url", invite_url.to_owned()),
    ]
}

/// Replace `{key}` placeholders. When `html`, values are HTML-escaped so a guest
/// name can't inject markup into the email body.
fn fill(template: &str, vars: &[(&str, String)], html: bool) -> String {
    let mut out = template.to_owned();
    for (k, v) in vars {
        let value = if html { html_escape(v) } else { v.clone() };
        out = out.replace(&format!("{{{k}}}"), &value);
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn ordinal(day: u32) -> String {
    let suffix = match (day % 10, day % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{day}{suffix}")
}

fn fmt_time(t: NaiveTime) -> String {
    let h24 = t.hour();
    let (h12, ap) = match h24 {
        0 => (12, "AM"),
        1..=11 => (h24, "AM"),
        12 => (12, "PM"),
        _ => (h24 - 12, "PM"),
    };
    format!("{}:{:02} {}", h12, t.minute(), ap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};

    fn sample() -> (Event, Guest) {
        let now = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let event = Event::new(
            uuid::Uuid::new_v4(),
            crate::domain::event::NewEvent {
                bride_name: "Hansika".into(),
                bride_family_name: "J".into(),
                groom_name: "Chirath".into(),
                groom_family_name: "N".into(),
                event_date: NaiveDate::from_ymd_opt(2026, 9, 25).unwrap(),
                start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                hall_name: "Kings Ballroom".into(),
                venue_name: "Kandy".into(),
                rsvp_by: NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
            },
            now,
        );
        let guest = Guest::new(
            event.id,
            crate::domain::guest::NewGuest {
                name: "Ravi <b>".into(),
                channel: crate::domain::guest::InviteChannel::EInvite,
                email: Some("r@x.com".into()),
                phone: None,
                max_party_size: 2,
            },
            now,
        );
        (event, guest)
    }

    #[test]
    fn renders_placeholders_and_link() {
        let (e, g) = sample();
        let t = MessageTemplates {
            email_html: "<p>{couple} — {guest_name} — {invite_url}</p>".into(),
            email_text: "{couple} {invite_url}".into(),
            email_subject: " {couple} \n".into(),
            whatsapp: "Hi {guest_name}, {date}, {invite_url}".into(),
        };
        let url = "https://x/i/AbC123";
        let email = t.render_email(&e, &g, url);
        assert_eq!(email.subject, "Hansika & Chirath");
        assert!(email.text.contains(url));
        // HTML body escapes the guest name's markup.
        assert!(email.html.contains("Ravi &lt;b&gt;"));
        assert!(email.html.contains(url));
        let wa = t.render_whatsapp(&e, &g, url);
        assert!(wa.contains("Ravi <b>")); // not escaped in plain text
        assert!(wa.contains("25 September 2026"));
        assert!(wa.contains(url));
    }
}
