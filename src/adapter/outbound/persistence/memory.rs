use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::model::User;
use crate::domain::port::outbound::UserRepository;

/// In-memory adapter for `UserRepository`. No external service required —
/// ideal for tests and local development. This is the payoff of the port/
/// adapter split: same trait, zero dependencies.
#[derive(Clone, Default)]
pub struct InMemoryUserRepository {
    users: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        let users = self.users.read().await;
        Ok(users.values().find(|u| u.email == email).cloned())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        Ok(self.users.read().await.get(&id).cloned())
    }

    async fn find_by_verification_token(&self, token: &str) -> Result<Option<User>, DomainError> {
        let users = self.users.read().await;
        Ok(users
            .values()
            .find(|u| u.verification_token.as_deref() == Some(token))
            .cloned())
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        let mut users = self.users.write().await;
        if users.values().any(|u| u.email == user.email) {
            return Err(DomainError::EmailAlreadyExists);
        }
        users.insert(user.id, user.clone());
        Ok(())
    }

    async fn update(&self, user: &User) -> Result<(), DomainError> {
        let mut users = self.users.write().await;
        users.insert(user.id, user.clone());
        Ok(())
    }
}
