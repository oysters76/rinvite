use async_trait::async_trait;

use crate::domain::error::DomainError;
use crate::domain::port::outbound::SmsClient;

/// Keyless-dev SMS client: logs the message instead of sending.
pub struct LogSmsClient;

#[async_trait]
impl SmsClient for LogSmsClient {
    async fn send_sms(&self, to_phone: &str, body: &str) -> Result<(), DomainError> {
        println!("[sms:log] to={to_phone}\n{body}\n---");
        Ok(())
    }
}
