use crate::domain::error::DomainError;

use super::{RenderedEmail, fill, load};

// Account-lifecycle email templates, embedded at compile time and overridable
// via env — the same pattern as the guest-invite `MessageTemplates`.
const DEFAULT_VERIFY_HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/messages/verify-email.html"
));
const DEFAULT_VERIFY_TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/messages/verify-email.txt"
));
const DEFAULT_VERIFY_SUBJECT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/messages/verify-email-subject.txt"
));
const DEFAULT_UPGRADE_HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/messages/upgrade-request.html"
));
const DEFAULT_UPGRADE_TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/messages/upgrade-request.txt"
));
const DEFAULT_UPGRADE_SUBJECT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/messages/upgrade-request-subject.txt"
));

/// Loads and renders the account-lifecycle emails: address verification and the
/// owner-facing upgrade-request notification.
#[derive(Clone)]
pub struct AccountTemplates {
    verify_html: String,
    verify_text: String,
    verify_subject: String,
    upgrade_html: String,
    upgrade_text: String,
    upgrade_subject: String,
}

impl AccountTemplates {
    pub fn from_env() -> Result<Self, DomainError> {
        Ok(Self {
            verify_html: load("VERIFY_EMAIL_TEMPLATE_HTML", DEFAULT_VERIFY_HTML)?,
            verify_text: load("VERIFY_EMAIL_TEMPLATE_TEXT", DEFAULT_VERIFY_TEXT)?,
            verify_subject: load("VERIFY_EMAIL_TEMPLATE_SUBJECT", DEFAULT_VERIFY_SUBJECT)?,
            upgrade_html: load("UPGRADE_EMAIL_TEMPLATE_HTML", DEFAULT_UPGRADE_HTML)?,
            upgrade_text: load("UPGRADE_EMAIL_TEMPLATE_TEXT", DEFAULT_UPGRADE_TEXT)?,
            upgrade_subject: load("UPGRADE_EMAIL_TEMPLATE_SUBJECT", DEFAULT_UPGRADE_SUBJECT)?,
        })
    }

    /// The verification email for a newly signed-up account.
    pub fn render_verification(&self, email: &str, verify_url: &str) -> RenderedEmail {
        let vars = vec![
            ("email", email.to_owned()),
            ("verify_url", verify_url.to_owned()),
        ];
        RenderedEmail {
            subject: fill(&self.verify_subject, &vars, false).trim().to_owned(),
            html: fill(&self.verify_html, &vars, true),
            text: fill(&self.verify_text, &vars, false),
        }
    }

    /// The owner notification sent when a user requests a plan upgrade.
    pub fn render_upgrade_request(
        &self,
        user_email: &str,
        current_plan: &str,
        requested_at: &str,
    ) -> RenderedEmail {
        let vars = vec![
            ("user_email", user_email.to_owned()),
            ("current_plan", current_plan.to_owned()),
            ("requested_at", requested_at.to_owned()),
        ];
        RenderedEmail {
            subject: fill(&self.upgrade_subject, &vars, false).trim().to_owned(),
            html: fill(&self.upgrade_html, &vars, true),
            text: fill(&self.upgrade_text, &vars, false),
        }
    }
}
