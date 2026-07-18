use async_trait::async_trait;

use crate::domain::error::DomainError;
use crate::domain::port::outbound::SmsClient;

/// [Twilio SMS](https://www.twilio.com/docs/sms) adapter — POSTs to the Messages
/// resource with HTTP basic auth. Shares the Twilio account with the WhatsApp
/// adapter; the only differences are the sender number and the lack of a
/// `whatsapp:` prefix / Meta content template.
#[allow(dead_code)] // retained for future SMS re-enable; phone uses WhatsApp
pub struct TwilioSms {
    http: reqwest::Client,
    account_sid: String,
    auth_token: String,
    /// SMS-capable sender number in E.164, e.g. `+14155238886`.
    from: String,
}

impl TwilioSms {
    pub fn new(account_sid: String, auth_token: String, from: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            account_sid,
            auth_token,
            from,
        }
    }
}

#[async_trait]
impl SmsClient for TwilioSms {
    async fn send_sms(&self, to_phone: &str, body: &str) -> Result<(), DomainError> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.account_sid
        );

        let form: Vec<(&str, String)> = vec![
            ("From", self.from.clone()),
            ("To", to_phone.to_owned()),
            ("Body", body.to_owned()),
        ];

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
