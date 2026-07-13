use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::model::User;
use crate::domain::plan::Plan;
use crate::domain::port::outbound::UserRepository;

fn row_to_user(row: &PgRow) -> Result<User, DomainError> {
    let e = |e: sqlx::Error| DomainError::Repository(e.to_string());
    let plan: String = row.try_get("plan").map_err(e)?;
    Ok(User {
        id: row.try_get("id").map_err(e)?,
        email: row.try_get("email").map_err(e)?,
        password_hash: row.try_get("password_hash").map_err(e)?,
        plan: Plan::parse(&plan)?,
        email_verified: row.try_get("email_verified").map_err(e)?,
        verification_token: row.try_get("verification_token").map_err(e)?,
        verification_expires_at: row
            .try_get::<Option<DateTime<Utc>>, _>("verification_expires_at")
            .map_err(e)?,
    })
}

/// Postgres adapter for `UserRepository`. Uses sqlx's runtime query API with
/// plain SQL — no ORM, and no live database needed at compile time. (If you
/// later switch to the compile-time-checked `sqlx::query!` macros, you'll need
/// a reachable DB or a prepared `.sqlx` cache when building.)
#[derive(Clone)]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        let row = sqlx::query(
            "SELECT id, email, password_hash, plan, email_verified, verification_token, \
             verification_expires_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        // Map the raw row into the domain type here, in the adapter, so sqlx
        // never leaks into the domain model.
        row.as_ref().map(row_to_user).transpose()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        let row = sqlx::query(
            "SELECT id, email, password_hash, plan, email_verified, verification_token, \
             verification_expires_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;
        row.as_ref().map(row_to_user).transpose()
    }

    async fn find_by_verification_token(&self, token: &str) -> Result<Option<User>, DomainError> {
        let row = sqlx::query(
            "SELECT id, email, password_hash, plan, email_verified, verification_token, \
             verification_expires_at FROM users WHERE verification_token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;
        row.as_ref().map(row_to_user).transpose()
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        let result = sqlx::query(
            "INSERT INTO users (id, email, password_hash, plan, email_verified, \
             verification_token, verification_expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(user.plan.as_str())
        .bind(user.email_verified)
        .bind(&user.verification_token)
        .bind(user.verification_expires_at)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            // Translate a unique-constraint violation into a domain error.
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                Err(DomainError::EmailAlreadyExists)
            }
            Err(e) => Err(DomainError::Repository(e.to_string())),
        }
    }

    async fn update(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE users SET plan = $2, email_verified = $3, verification_token = $4, \
             verification_expires_at = $5 WHERE id = $1",
        )
        .bind(user.id)
        .bind(user.plan.as_str())
        .bind(user.email_verified)
        .bind(&user.verification_token)
        .bind(user.verification_expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(())
    }
}
