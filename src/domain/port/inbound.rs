use async_trait::async_trait;

use super::super::error::DomainError;

/// Returned to a client after a successful signup/login.
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub value: String,
}

// ===== Inbound port (driving side) =========================================
// What the outside world is allowed to ask of our application. HTTP handlers,
// a CLI, or a message consumer all call *this* trait and nothing deeper.

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn signup(&self, email: &str, password: &str) -> Result<AuthToken, DomainError>;
    async fn login(&self, email: &str, password: &str) -> Result<AuthToken, DomainError>;
}
