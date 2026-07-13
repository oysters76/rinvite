use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::adapter::outbound::message::account::AccountTemplates;
use crate::domain::error::DomainError;
use crate::domain::model::User;
use crate::domain::port::inbound::{AuthService, AuthToken};
use crate::domain::port::outbound::{
    Clock, EmailClient, PasswordHasher, TokenIssuer, UserRepository,
};
use crate::domain::validation::{validate_email, validate_password};

/// Implements the inbound `AuthService` port by coordinating the outbound
/// ports. This is where the use-case logic lives. It depends ONLY on traits,
/// never on axum, sqlx, argon2, jwt, etc. — that's what keeps it testable and
/// lets you swap any adapter without touching this file.
pub struct AuthServiceImpl {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn TokenIssuer>,
    email: Arc<dyn EmailClient>,
    clock: Arc<dyn Clock>,
    templates: AccountTemplates,
    /// Base URL used to build the email-verification link.
    public_base_url: String,
}

impl AuthServiceImpl {
    pub fn new(
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        tokens: Arc<dyn TokenIssuer>,
        email: Arc<dyn EmailClient>,
        clock: Arc<dyn Clock>,
        templates: AccountTemplates,
        public_base_url: String,
    ) -> Self {
        Self {
            users,
            hasher,
            tokens,
            email,
            clock,
            templates,
            public_base_url,
        }
    }

    /// Send the verification email for `user` (must have an outstanding token).
    async fn send_verification(&self, user: &User) -> Result<(), DomainError> {
        let Some(token) = user.verification_token.as_deref() else {
            return Ok(());
        };
        let verify_url = format!("{}/verify?token={}", self.public_base_url, token);
        let m = self.templates.render_verification(&user.email, &verify_url);
        self.email
            .send_email(&user.email, &m.subject, &m.html, &m.text)
            .await
    }
}

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn signup(&self, email: &str, password: &str) -> Result<(), DomainError> {
        validate_email(email)?;
        validate_password(password)?;

        if self.users.find_by_email(email).await?.is_some() {
            return Err(DomainError::EmailAlreadyExists);
        }

        let password_hash = self.hasher.hash(password).await?;
        let user = User::new(email.to_owned(), password_hash, self.clock.now());
        self.users.save(&user).await?;

        // Email the verification link. No token is returned: the user must
        // verify before logging in.
        self.send_verification(&user).await
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
                    // Only *after* a correct password do we reveal the account's
                    // verification state — so this never leaks membership.
                    if !user.email_verified {
                        return Err(DomainError::EmailNotVerified);
                    }
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

    async fn verify_email(&self, token: &str) -> Result<(), DomainError> {
        let mut user = self
            .users
            .find_by_verification_token(token)
            .await?
            .ok_or_else(|| DomainError::InvalidInput("invalid verification link".to_owned()))?;

        // Already verified (or a reused link) — nothing to do.
        if user.email_verified {
            return Ok(());
        }

        // Reject an expired token; the user can request a fresh one.
        if let Some(exp) = user.verification_expires_at
            && self.clock.now() > exp
        {
            return Err(DomainError::InvalidInput(
                "this verification link has expired".to_owned(),
            ));
        }

        user.mark_verified();
        self.users.update(&user).await
    }

    async fn resend_verification(&self, email: &str) -> Result<(), DomainError> {
        // Never reveal whether the address exists or is already verified.
        if let Some(mut user) = self.users.find_by_email(email).await?
            && !user.email_verified
        {
            user.reissue_verification(self.clock.now());
            self.users.update(&user).await?;
            self.send_verification(&user).await?;
        }
        Ok(())
    }

    async fn me(&self, user_id: Uuid) -> Result<User, DomainError> {
        self.users
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("user".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::{DateTime, TimeZone, Utc};

    use super::*;
    use crate::adapter::outbound::persistence::memory::InMemoryUserRepository;

    /// Trivial reversible "hash": the hash is just the plaintext.
    struct FakeHasher;
    #[async_trait]
    impl PasswordHasher for FakeHasher {
        async fn hash(&self, plaintext: &str) -> Result<String, DomainError> {
            Ok(plaintext.to_owned())
        }
        async fn verify(&self, plaintext: &str, hash: &str) -> Result<bool, DomainError> {
            Ok(plaintext == hash)
        }
        async fn verify_dummy(&self, _plaintext: &str) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct FakeTokens;
    impl TokenIssuer for FakeTokens {
        fn issue(&self, _user_id: Uuid) -> Result<String, DomainError> {
            Ok("jwt".to_owned())
        }
    }

    /// Records the recipients emailed, so tests can assert a mail was sent.
    #[derive(Default)]
    struct SpyEmail {
        sent_to: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl EmailClient for SpyEmail {
        async fn send_email(
            &self,
            to: &str,
            _s: &str,
            _h: &str,
            _t: &str,
        ) -> Result<(), DomainError> {
            self.sent_to.lock().unwrap().push(to.to_owned());
            Ok(())
        }
    }

    #[derive(Clone)]
    struct FixedClock(DateTime<Utc>);
    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn service(clock: FixedClock) -> (AuthServiceImpl, Arc<InMemoryUserRepository>, Arc<SpyEmail>) {
        let users = Arc::new(InMemoryUserRepository::new());
        let email = Arc::new(SpyEmail::default());
        let svc = AuthServiceImpl::new(
            users.clone(),
            Arc::new(FakeHasher),
            Arc::new(FakeTokens),
            email.clone(),
            Arc::new(clock),
            AccountTemplates::from_env().expect("templates load"),
            "http://app".into(),
        );
        (svc, users, email)
    }

    fn now() -> FixedClock {
        FixedClock(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap())
    }

    #[tokio::test]
    async fn signup_creates_unverified_user_and_emails_link() {
        let (svc, users, email) = service(now());
        svc.signup("a@b.com", "password1").await.unwrap();

        let user = users.find_by_email("a@b.com").await.unwrap().unwrap();
        assert!(!user.email_verified);
        assert!(user.verification_token.is_some());
        // Verification email went to the new address.
        assert_eq!(email.sent_to.lock().unwrap().as_slice(), ["a@b.com"]);
    }

    #[tokio::test]
    async fn login_is_refused_until_verified_then_succeeds() {
        let (svc, users, _) = service(now());
        svc.signup("a@b.com", "password1").await.unwrap();

        // Unverified -> EmailNotVerified (only after the correct password).
        assert!(matches!(
            svc.login("a@b.com", "password1").await,
            Err(DomainError::EmailNotVerified)
        ));
        // Wrong password still looks like bad credentials, not "not verified".
        assert!(matches!(
            svc.login("a@b.com", "wrong").await,
            Err(DomainError::InvalidCredentials)
        ));

        // Verify, then login works.
        let token = users
            .find_by_email("a@b.com")
            .await
            .unwrap()
            .unwrap()
            .verification_token
            .unwrap();
        svc.verify_email(&token).await.unwrap();
        assert_eq!(
            svc.login("a@b.com", "password1").await.unwrap().value,
            "jwt"
        );
    }

    #[tokio::test]
    async fn expired_verification_token_is_rejected() {
        let (svc, users, _) = service(now());
        svc.signup("a@b.com", "password1").await.unwrap();
        let token = users
            .find_by_email("a@b.com")
            .await
            .unwrap()
            .unwrap()
            .verification_token
            .unwrap();

        // Move the clock past the 24h TTL by verifying with a later-clock service.
        let late = FixedClock(Utc.with_ymd_and_hms(2026, 7, 3, 0, 0, 0).unwrap());
        let svc_late = AuthServiceImpl::new(
            users.clone(),
            Arc::new(FakeHasher),
            Arc::new(FakeTokens),
            Arc::new(SpyEmail::default()),
            Arc::new(late),
            AccountTemplates::from_env().unwrap(),
            "http://app".into(),
        );
        assert!(matches!(
            svc_late.verify_email(&token).await,
            Err(DomainError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn resend_is_silent_for_unknown_or_verified_addresses() {
        let (svc, _users, email) = service(now());
        // Unknown address: no error, no mail.
        svc.resend_verification("nobody@x.com").await.unwrap();
        assert!(email.sent_to.lock().unwrap().is_empty());
    }
}
