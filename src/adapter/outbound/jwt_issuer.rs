use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::port::outbound::TokenIssuer;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// Subject — we store the user id here.
    sub: String,
    /// Expiry as a Unix timestamp (seconds).
    exp: usize,
}

/// JWT adapter for the `TokenIssuer` port.
pub struct JwtIssuer {
    secret: String,
    ttl_seconds: u64,
}

impl JwtIssuer {
    pub fn new(secret: String, ttl_seconds: u64) -> Self {
        Self {
            secret,
            ttl_seconds,
        }
    }
}

impl TokenIssuer for JwtIssuer {
    fn issue(&self, user_id: Uuid) -> Result<String, DomainError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| DomainError::TokenCreation(e.to_string()))?
            .as_secs();

        let claims = Claims {
            sub: user_id.to_string(),
            exp: (now + self.ttl_seconds) as usize,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| DomainError::TokenCreation(e.to_string()))
    }
}
