use std::sync::LazyLock;

use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
// Bring argon2's own traits into scope anonymously (`as _`) so their methods
// are callable without clashing with our domain `PasswordHasher` trait name.
use argon2::{Argon2, PasswordHash, PasswordHasher as _, PasswordVerifier as _};
use async_trait::async_trait;

use crate::domain::error::DomainError;
use crate::domain::port::outbound::PasswordHasher;

/// A valid argon2 hash built once at first use, with the same parameters real
/// hashes use. `verify_dummy` verifies against this so an unknown email costs
/// the same wall-clock time as a wrong password.
static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(b"dummy-password-for-timing-equalization", &salt)
        .expect("dummy hash must build")
        .to_string()
});

/// Argon2id adapter for the `PasswordHasher` port.
///
/// argon2 hashing is CPU-bound and blocking, so every call is offloaded to
/// `tokio::task::spawn_blocking` to keep the async runtime's worker threads
/// free for I/O.
pub struct Argon2Hasher;

fn hash_blocking(plaintext: &str) -> Result<String, DomainError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plaintext.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| DomainError::Hashing(e.to_string()))
}

fn verify_blocking(plaintext: &str, hash: &str) -> Result<bool, DomainError> {
    let parsed = PasswordHash::new(hash).map_err(|e| DomainError::Hashing(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(plaintext.as_bytes(), &parsed)
        .is_ok())
}

#[async_trait]
impl PasswordHasher for Argon2Hasher {
    async fn hash(&self, plaintext: &str) -> Result<String, DomainError> {
        let plaintext = plaintext.to_owned();
        tokio::task::spawn_blocking(move || hash_blocking(&plaintext))
            .await
            .map_err(|e| DomainError::Hashing(format!("hashing task failed: {e}")))?
    }

    async fn verify(&self, plaintext: &str, hash: &str) -> Result<bool, DomainError> {
        let plaintext = plaintext.to_owned();
        let hash = hash.to_owned();
        tokio::task::spawn_blocking(move || verify_blocking(&plaintext, &hash))
            .await
            .map_err(|e| DomainError::Hashing(format!("verify task failed: {e}")))?
    }

    async fn verify_dummy(&self, plaintext: &str) -> Result<(), DomainError> {
        let plaintext = plaintext.to_owned();
        tokio::task::spawn_blocking(move || verify_blocking(&plaintext, &DUMMY_HASH))
            .await
            .map_err(|e| DomainError::Hashing(format!("verify task failed: {e}")))??;
        Ok(())
    }
}
