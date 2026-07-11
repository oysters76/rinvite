mod adapter;
mod application;
mod domain;

use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use adapter::inbound::http::{AppState, routes};
use adapter::outbound::argon2_hasher::Argon2Hasher;
use adapter::outbound::jwt_issuer::JwtIssuer;
use adapter::outbound::persistence::memory::InMemoryUserRepository;
use adapter::outbound::persistence::postgres::PostgresUserRepository;
use application::auth_service::AuthServiceImpl;
use domain::port::inbound::AuthService;
use domain::port::outbound::{PasswordHasher, TokenIssuer, UserRepository};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ===== Configuration (read from env; add a .env loader if you like) =====
    // The signing key is required — never fall back to a baked-in default, or a
    // misconfigured deploy would issue tokens anyone can forge. Fail fast.
    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET must be set")?;
    if jwt_secret.len() < 32 {
        return Err("JWT_SECRET must be at least 32 bytes".into());
    }
    let token_ttl_secs: u64 = 24 * 60 * 60; // 1 day

    // ===== Outbound adapters (driven side) =================================
    let hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2Hasher);
    let tokens: Arc<dyn TokenIssuer> = Arc::new(JwtIssuer::new(jwt_secret, token_ttl_secs));

    // Choose the repository adapter at runtime. Same `dyn UserRepository`, so
    // nothing downstream cares which one it gets.
    let users: Arc<dyn UserRepository> = match std::env::var("DATABASE_URL") {
        Ok(url) => {
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await?;

            // For a starter we create the table on boot. In a real project run
            // migrations instead (see migrations/ and `sqlx migrate run`).
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS users (
                    id UUID PRIMARY KEY,
                    email TEXT NOT NULL UNIQUE,
                    password_hash TEXT NOT NULL
                )",
            )
            .execute(&pool)
            .await?;

            println!("Using Postgres repository");
            Arc::new(PostgresUserRepository::new(pool))
        }
        Err(_) => {
            println!("DATABASE_URL not set — using in-memory repository");
            Arc::new(InMemoryUserRepository::new())
        }
    };

    // ===== Application service (implements the inbound port) ===============
    let auth: Arc<dyn AuthService> = Arc::new(AuthServiceImpl::new(users, hasher, tokens));

    // ===== Inbound adapter (HTTP) + serve =================================
    let app = routes(AppState { auth });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
