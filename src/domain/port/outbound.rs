use async_trait::async_trait;
use uuid::Uuid;

use super::super::error::DomainError;
use super::super::model::User;

// ===== Outbound ports (driven side) ========================================
// What our application needs *from* the outside world. Each of these is
// implemented by one or more adapters (Postgres, in-memory, argon2, jwt, ...).

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
}

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, plaintext: &str) -> Result<String, DomainError>;
    async fn verify(&self, plaintext: &str, hash: &str) -> Result<bool, DomainError>;

    /// Verify against a fixed internal hash to equalize timing when no user
    /// exists — defeats account enumeration via response latency. The concrete
    /// dummy hash lives in the adapter, so the application never learns the
    /// hash format.
    async fn verify_dummy(&self, plaintext: &str) -> Result<(), DomainError>;
}

pub trait TokenIssuer: Send + Sync {
    fn issue(&self, user_id: Uuid) -> Result<String, DomainError>;
}
