use async_trait::async_trait;

use crate::domain::error::DomainError;
use crate::domain::port::outbound::WhatsAppClient;

/// [Twilio WhatsApp](https://www.twilio.com/docs/whatsapp) adapter — POSTs to the
/// Messages resource with HTTP basic auth.
///
/// Freeform `Body` works in the sandbox / 24-hour session window. For
/// business-initiated production messages set `content_sid` (a Meta-approved
/// template); the adapter then sends template mode, passing the rendered body as
/// content variable `1`.
#[allow(dead_code)] // retained for future WhatsApp re-enable; phone uses SMS
pub struct TwilioWhatsApp {
    http: reqwest::Client,
    account_sid: String,
    auth_token: String,
    /// WhatsApp-enabled sender number in E.164, e.g. `+14155238886`.
    from: String,
    content_sid: Option<String>,
}

impl TwilioWhatsApp {
    pub fn new(
        account_sid: String,
        auth_token: String,
        from: String,
        content_sid: Option<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            account_sid,
            auth_token,
            from,
            content_sid,
        }
    }
}

#[async_trait]
impl WhatsAppClient for TwilioWhatsApp {
    async fn send_whatsapp(&self, to_phone: &str, body: &str) -> Result<(), DomainError> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.account_sid
        );

        let mut form: Vec<(&str, String)> = vec![
            ("From", format!("whatsapp:{}", self.from)),
            ("To", format!("whatsapp:{to_phone}")),
        ];
        match &self.content_sid {
            Some(sid) => {
                form.push(("ContentSid", sid.clone()));
                form.push((
                    "ContentVariables",
                    serde_json::json!({ "1": body }).to_string(),
                ));
            }
            None => form.push(("Body", body.to_owned())),
        }

        let res = self
            .http
            .post(url)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .form(&form)
            .send()
            .await
            .map_err(|e| DomainError::Repository(format!("twilio request failed: {e}")))?;

        if !res.status().is_success() {
            let status = res.status();
            let b = res.text().await.unwrap_or_default();
            return Err(DomainError::Repository(format!(
                "twilio error {status}: {b}"
            )));
        }
        Ok(())
    }
}
