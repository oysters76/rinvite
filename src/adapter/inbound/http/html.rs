use std::sync::Arc;

use chrono::{Datelike, NaiveTime, Timelike};
use serde::Serialize;

use crate::domain::error::DomainError;
use crate::domain::port::inbound::InviteView;

/// The e-invite web page template, embedded at compile time. Editing
/// `assets/einvite/template.html` (or pointing `EINVITE_TEMPLATE` at a copy)
/// restyles the page without touching Rust.
const DEFAULT_TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/einvite/template.html"
));

/// Load the template once at startup: the `EINVITE_TEMPLATE` file if set,
/// otherwise the embedded default.
pub fn load_template() -> Result<Arc<str>, DomainError> {
    match std::env::var("EINVITE_TEMPLATE") {
        Ok(path) => {
            let body = std::fs::read_to_string(&path).map_err(|e| {
                DomainError::Repository(format!("cannot read EINVITE_TEMPLATE {path}: {e}"))
            })?;
            Ok(Arc::from(body))
        }
        Err(_) => Ok(Arc::from(DEFAULT_TEMPLATE)),
    }
}

/// Data injected into the template's `#invite-data` JSON block. Field names are
/// camelCased to match what the template's JS reads.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InvitePageData {
    bride_name: String,
    groom_name: String,
    monogram: String,
    families: String,
    greeting: String,
    date_big: String,
    date_year: String,
    time_text: String,
    venue_name: String,
    hall_name: String,
    /// Optional labelled Poruwa line; `None` when unset so the page hides it.
    poruwa_text: Option<String>,
    rsvp_by_text: String,
    footer: String,
    rsvp: RsvpData,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RsvpData {
    endpoint: String,
    max_party_size: u16,
    status: String,
    party_size: Option<u16>,
    closed: bool,
}

/// Render the e-invite page by injecting this guest's data into the template.
pub fn render_invite_page(template: &str, view: &InviteView, token: &str) -> String {
    let e = &view.event;
    let data = InvitePageData {
        bride_name: e.bride_name.clone(),
        groom_name: e.groom_name.clone(),
        monogram: format!("{}&{}", initial(&e.bride_name), initial(&e.groom_name)),
        families: format!(
            "Together with the {} & {} families",
            e.bride_family_name, e.groom_family_name
        ),
        greeting: format!("Dear {},", view.guest_name),
        date_big: format!(
            "{} of {}",
            ordinal(e.event_date.day()),
            e.event_date.format("%B")
        ),
        date_year: e.event_date.format("%Y").to_string(),
        time_text: format!(
            "From {} to {}",
            fmt_time(e.start_time),
            fmt_time(e.end_time)
        ),
        venue_name: e.venue_name.clone(),
        hall_name: e.hall_name.clone(),
        poruwa_text: e
            .poruwa_ceremony_time
            .map(|t| format!("Poruwa Ceremony at {}", fmt_time(t))),
        rsvp_by_text: format!(
            "RSVP by {} {}",
            ordinal(e.rsvp_by.day()),
            e.rsvp_by.format("%B")
        ),
        footer: format!(
            "{} & {} · {}",
            e.bride_name,
            e.groom_name,
            e.event_date.format("%B %Y")
        ),
        rsvp: RsvpData {
            endpoint: format!("/i/{token}/rsvp"),
            max_party_size: view.max_party_size,
            status: view.rsvp_status.as_str().to_owned(),
            party_size: view.party_size,
            closed: view.rsvp_closed,
        },
    };

    let json = serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_owned());
    template.replace("__INVITE_JSON__", &escape_json_for_script(&json))
}

/// Escape a JSON string for safe embedding inside an HTML `<script>` block:
/// `<`, `>`, `&`, and the JS line terminators become `\uXXXX` escapes (still
/// valid JSON) so a guest name can't break out of the tag. This is the XSS
/// guard now that guest/event text is injected as JSON.
fn escape_json_for_script(json: &str) -> String {
    json.replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

fn initial(name: &str) -> char {
    name.trim()
        .chars()
        .next()
        .unwrap_or('•')
        .to_ascii_uppercase()
}

/// Ordinal day-of-month, e.g. 25 -> "25th", 22 -> "22nd".
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

/// Friendly 12-hour time, e.g. "10:00 AM", "3:30 PM".
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

    #[test]
    fn escaping_neutralizes_script_breakout() {
        let escaped = escape_json_for_script(r#"{"greeting":"Dear </script><b>x</b>,"}"#);
        assert!(!escaped.contains("</script>"));
        assert!(escaped.contains("\\u003c"));
    }

    #[test]
    fn time_and_ordinal_formats() {
        assert_eq!(
            fmt_time(NaiveTime::from_hms_opt(10, 0, 0).unwrap()),
            "10:00 AM"
        );
        assert_eq!(
            fmt_time(NaiveTime::from_hms_opt(15, 30, 0).unwrap()),
            "3:30 PM"
        );
        assert_eq!(ordinal(25), "25th");
        assert_eq!(ordinal(1), "1st");
    }
}
