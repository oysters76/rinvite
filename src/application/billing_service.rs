use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::adapter::outbound::message::account::AccountTemplates;
use crate::domain::error::DomainError;
use crate::domain::port::inbound::BillingService;
use crate::domain::port::outbound::{Clock, EmailClient, UserRepository};

/// Implements the inbound `BillingService` port. A user's upgrade request is
/// just an email to the app owner — plan changes are applied manually.
pub struct BillingServiceImpl {
    users: Arc<dyn UserRepository>,
    email: Arc<dyn EmailClient>,
    clock: Arc<dyn Clock>,
    templates: AccountTemplates,
    /// Where upgrade-request notifications are delivered (the app owner).
    notify_email: String,
}

impl BillingServiceImpl {
    pub fn new(
        users: Arc<dyn UserRepository>,
        email: Arc<dyn EmailClient>,
        clock: Arc<dyn Clock>,
        templates: AccountTemplates,
        notify_email: String,
    ) -> Self {
        Self {
            users,
            email,
            clock,
            templates,
            notify_email,
        }
    }
}

#[async_trait]
impl BillingService for BillingServiceImpl {
    async fn request_upgrade(&self, user_id: Uuid) -> Result<(), DomainError> {
        let user = self
            .users
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("user".to_owned()))?;

        let requested_at = self.clock.now().format("%Y-%m-%d %H:%M UTC").to_string();
        let m =
            self.templates
                .render_upgrade_request(&user.email, user.plan.as_str(), &requested_at);
        self.email
            .send_email(&self.notify_email, &m.subject, &m.html, &m.text)
            .await
    }
}
