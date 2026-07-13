use async_trait::async_trait;

use crate::domain::error::DomainError;
use crate::domain::port::outbound::WhatsAppClient;

/// Keyless-dev WhatsApp client: logs the message instead of sending.
pub struct LogWhatsAppClient;

#[async_trait]
impl WhatsAppClient for LogWhatsAppClient {
    async fn send_whatsapp(&self, to_phone: &str, body: &str) -> Result<(), DomainError> {
        println!("[whatsapp:log] to={to_phone}\n{body}\n---");
        Ok(())
    }
}
