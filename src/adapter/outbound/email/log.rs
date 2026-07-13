use async_trait::async_trait;

use crate::domain::error::DomainError;
use crate::domain::port::outbound::EmailClient;

/// Keyless-dev email client: logs the message instead of sending. Used when no
/// email provider is configured so the app (and the dashboard's Send) still work.
pub struct LogEmailClient;

#[async_trait]
impl EmailClient for LogEmailClient {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        _html: &str,
        text: &str,
    ) -> Result<(), DomainError> {
        println!("[email:log] to={to} subject=\"{subject}\"\n{text}\n---");
        Ok(())
    }
}
