use async_trait::async_trait;
use serde_json::json;

use crate::domain::error::DomainError;
use crate::domain::port::outbound::EmailClient;

/// [Resend](https://resend.com) email adapter — a single JSON POST.
pub struct ResendClient {
    http: reqwest::Client,
    api_key: String,
    /// Sender, e.g. `Rinvite <invites@yourdomain>`.
    from: String,
}

impl ResendClient {
    pub fn new(api_key: String, from: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
            from,
        }
    }
}

#[async_trait]
impl EmailClient for ResendClient {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html: &str,
        text: &str,
    ) -> Result<(), DomainError> {
        let res = self
            .http
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&json!({ "from": self.from, "to": [to], "subject": subject, "html": html, "text": text }))
            .send()
            .await
            .map_err(|e| DomainError::Repository(format!("resend request failed: {e}")))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(DomainError::Repository(format!(
                "resend error {status}: {body}"
            )));
        }
        Ok(())
    }
}
