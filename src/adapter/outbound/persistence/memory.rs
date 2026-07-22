use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::approval::ApprovalStatus;
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

    async fn delete_stale(&self, cutoff: DateTime<Utc>) -> Result<u64, DomainError> {
        let mut users = self.users.write().await;
        let before = users.len();
        // Keep fully-approved accounts (any age) and anything created on/after
        // the cutoff; drop stale, not-fully-approved accounts.
        users.retain(|_, u| {
            let fully_approved = u.email_verified && u.approval_status == ApprovalStatus::Approved;
            fully_approved || u.created_at >= cutoff
        });
        Ok((before - users.len()) as u64)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    fn user_at(email: &str, created_at: DateTime<Utc>) -> User {
        User::new(email.to_owned(), "hash".to_owned(), created_at)
    }

    #[tokio::test]
    async fn delete_stale_removes_only_old_unapproved_accounts() {
        let repo = InMemoryUserRepository::new();
        let cutoff = Utc.with_ymd_and_hms(2026, 7, 8, 0, 0, 0).unwrap();

        // Old + still pending -> deleted.
        let old_pending = user_at(
            "old@x.com",
            Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        );
        // Old but fully approved -> kept.
        let mut old_approved = user_at(
            "keep@x.com",
            Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        );
        old_approved.email_verified = true;
        old_approved.approve();
        // Recent + pending (created after cutoff) -> kept.
        let recent_pending = user_at(
            "new@x.com",
            Utc.with_ymd_and_hms(2026, 7, 20, 0, 0, 0).unwrap(),
        );

        repo.save(&old_pending).await.unwrap();
        repo.save(&old_approved).await.unwrap();
        repo.save(&recent_pending).await.unwrap();

        let removed = repo.delete_stale(cutoff).await.unwrap();
        assert_eq!(removed, 1);
        assert!(repo.find_by_email("old@x.com").await.unwrap().is_none());
        assert!(repo.find_by_email("keep@x.com").await.unwrap().is_some());
        assert!(repo.find_by_email("new@x.com").await.unwrap().is_some());
    }
}
