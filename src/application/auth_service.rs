use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::model::User;
use crate::domain::port::inbound::{AuthService, AuthToken};
use crate::domain::port::outbound::{PasswordHasher, TokenIssuer, UserRepository};
use crate::domain::validation::{validate_email, validate_password};

/// Implements the inbound `AuthService` port by coordinating the outbound
/// ports. This is where the use-case logic lives. It depends ONLY on traits,
/// never on axum, sqlx, argon2, jwt, etc. — that's what keeps it testable and
/// lets you swap any adapter without touching this file.
pub struct AuthServiceImpl {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn TokenIssuer>,
}

impl AuthServiceImpl {
    pub fn new(
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        tokens: Arc<dyn TokenIssuer>,
    ) -> Self {
        Self {
            users,
            hasher,
            tokens,
        }
    }
}

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn signup(&self, email: &str, password: &str) -> Result<AuthToken, DomainError> {
        validate_email(email)?;
        validate_password(password)?;

        if self.users.find_by_email(email).await?.is_some() {
            return Err(DomainError::EmailAlreadyExists);
        }

        let password_hash = self.hasher.hash(password).await?;
        let user = User::new(email.to_owned(), password_hash);
        self.users.save(&user).await?;

        let token = self.tokens.issue(user.id)?;
        Ok(AuthToken { value: token })
    }

    async fn login(&self, email: &str, password: &str) -> Result<AuthToken, DomainError> {
        // Return the SAME error, and spend the SAME wall-clock time, whether the
        // email is unknown or the password is wrong — otherwise the latency gap
        // leaks which emails are registered. We deliberately do NOT validate the
        // input format here: a malformed email should look identical to a
        // non-existent one, not produce a distinguishing 400.
        match self.users.find_by_email(email).await? {
            Some(user) => {
                if self.hasher.verify(password, &user.password_hash).await? {
                    let token = self.tokens.issue(user.id)?;
                    Ok(AuthToken { value: token })
                } else {
                    Err(DomainError::InvalidCredentials)
                }
            }
            None => {
                // Burn an equivalent argon2 verify so timing matches the
                // wrong-password path above.
                self.hasher.verify_dummy(password).await?;
                Err(DomainError::InvalidCredentials)
            }
        }
    }

    async fn me(&self, user_id: Uuid) -> Result<User, DomainError> {
        self.users
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("user".to_owned()))
    }
}
