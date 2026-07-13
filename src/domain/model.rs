use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::plan::Plan;

/// How long an email-verification token stays valid after it is issued.
pub const VERIFICATION_TTL: Duration = Duration::hours(24);

/// Core domain entity. No framework, database, or transport types leak in here.
#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    /// The account's subscription plan (defaults to `Free` at signup).
    pub plan: Plan,
    /// True once the owner has confirmed their email via the verification link.
    pub email_verified: bool,
    /// The outstanding verification token, cleared once the email is verified.
    pub verification_token: Option<String>,
    /// When the outstanding token stops being accepted.
    pub verification_expires_at: Option<DateTime<Utc>>,
}

impl User {
    /// Create a brand new, unverified `Free` user with a freshly generated id
    /// and an email-verification token valid until `now + VERIFICATION_TTL`.
    pub fn new(email: String, password_hash: String, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            email,
            password_hash,
            plan: Plan::Free,
            email_verified: false,
            verification_token: Some(verification_token()),
            verification_expires_at: Some(now + VERIFICATION_TTL),
        }
    }

    /// Issue a fresh verification token (used when resending the email).
    pub fn reissue_verification(&mut self, now: DateTime<Utc>) {
        self.verification_token = Some(verification_token());
        self.verification_expires_at = Some(now + VERIFICATION_TTL);
    }

    /// Mark the email verified and clear the outstanding token.
    pub fn mark_verified(&mut self) {
        self.email_verified = true;
        self.verification_token = None;
        self.verification_expires_at = None;
    }
}

/// A long, URL-safe, unguessable verification token (32 base62 chars). Longer
/// than the guest invite token because it gates account access. Each character
/// is drawn from a fresh random byte across two v4 UUIDs (256 bits of source).
fn verification_token() -> String {
    const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes
        .iter()
        .map(|b| ALPHABET[(*b as usize) % 62] as char)
        .collect()
}
