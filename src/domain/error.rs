use thiserror::Error;

/// The single error type the domain speaks. Adapters map their own failures
/// (sqlx errors, argon2 errors, ...) into these variants, and the HTTP adapter
/// maps these into status codes. The domain never sees adapter-specific errors.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("email already registered")]
    EmailAlreadyExists,

    #[error("invalid email or password")]
    InvalidCredentials,

    // The message describes the caller's own input, so it is safe to return.
    #[error("{0}")]
    InvalidInput(String),

    #[error("repository error: {0}")]
    Repository(String),

    #[error("password hashing error: {0}")]
    Hashing(String),

    #[error("token creation error: {0}")]
    TokenCreation(String),
}
