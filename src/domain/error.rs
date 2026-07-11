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

    // The named thing does not exist — OR the caller isn't allowed to see it.
    // We deliberately fold "forbidden" into "not found" for owner-scoped reads
    // so we never confirm that someone else's resource exists.
    #[error("{0} not found")]
    NotFound(String),

    #[error("the RSVP deadline has passed")]
    RsvpClosed,

    #[error("party size exceeds the maximum for this guest")]
    PartySizeExceeded,

    #[error("authentication required")]
    Unauthorized,

    #[error("repository error: {0}")]
    Repository(String),

    #[error("password hashing error: {0}")]
    Hashing(String),

    #[error("token creation error: {0}")]
    TokenCreation(String),

    #[error("pdf generation error: {0}")]
    Pdf(String),
}
