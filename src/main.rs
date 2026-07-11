mod adapter;
mod application;
mod domain;

use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use adapter::inbound::http::{AppState, routes};
use adapter::outbound::argon2_hasher::Argon2Hasher;
use adapter::outbound::clock::SystemClock;
use adapter::outbound::jwt_issuer::JwtIssuer;
use adapter::outbound::manual_sender::ManualSender;
use adapter::outbound::pdf::TemplatePdfRenderer;
use adapter::outbound::persistence::events_memory::InMemoryEventStore;
use adapter::outbound::persistence::events_postgres::PostgresEventStore;
use adapter::outbound::persistence::memory::InMemoryUserRepository;
use adapter::outbound::persistence::postgres::PostgresUserRepository;
use application::auth_service::AuthServiceImpl;
use application::event_service::EventServiceImpl;
use application::invite_service::InviteServiceImpl;
use domain::port::inbound::{AuthService, EventService, InviteService};
use domain::port::outbound::{
    Clock, EventRepository, GuestRepository, InvitePdfRenderer, InviteSender, PasswordHasher,
    TokenIssuer, TokenVerifier, UserRepository,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ===== Configuration =====================================================
    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET must be set")?;
    if jwt_secret.len() < 32 {
        return Err("JWT_SECRET must be at least 32 bytes".into());
    }
    let token_ttl_secs: u64 = 24 * 60 * 60; // 1 day
    // Base URL used to build shareable invite links.
    let public_base_url =
        std::env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_owned());

    // ===== Outbound adapters (driven side) ==================================
    let hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2Hasher);
    let jwt = Arc::new(JwtIssuer::new(jwt_secret, token_ttl_secs));
    let tokens: Arc<dyn TokenIssuer> = jwt.clone();
    let verifier: Arc<dyn TokenVerifier> = jwt.clone();
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let pdf: Arc<dyn InvitePdfRenderer> = Arc::new(TemplatePdfRenderer::from_env()?);
    // No-op delivery for now; the port is the seam for a future WhatsApp/bulk sender.
    let sender: Arc<dyn InviteSender> = Arc::new(ManualSender);

    // Repositories: pick Postgres or in-memory at runtime. The event store backs
    // both the event and guest repository ports, so one instance serves both.
    let (users, events_repo, guests_repo): (
        Arc<dyn UserRepository>,
        Arc<dyn EventRepository>,
        Arc<dyn GuestRepository>,
    ) = match std::env::var("DATABASE_URL") {
        Ok(url) => {
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await?;
            sqlx::migrate!("./migrations").run(&pool).await?;
            let store = Arc::new(PostgresEventStore::new(pool.clone()));
            println!("Using Postgres repositories");
            (
                Arc::new(PostgresUserRepository::new(pool)),
                store.clone(),
                store,
            )
        }
        Err(_) => {
            let store = Arc::new(InMemoryEventStore::new());
            println!("DATABASE_URL not set — using in-memory repositories");
            (
                Arc::new(InMemoryUserRepository::new()),
                store.clone(),
                store,
            )
        }
    };

    // ===== Application services (implement the inbound ports) ================
    let auth: Arc<dyn AuthService> = Arc::new(AuthServiceImpl::new(users, hasher, tokens));
    let events: Arc<dyn EventService> = Arc::new(EventServiceImpl::new(
        events_repo.clone(),
        guests_repo.clone(),
        pdf,
        sender,
        clock.clone(),
        public_base_url.clone(),
    ));
    let invites: Arc<dyn InviteService> =
        Arc::new(InviteServiceImpl::new(events_repo, guests_repo, clock));

    // ===== Inbound adapter (HTTP) + serve ===================================
    let app = routes(AppState {
        auth,
        events,
        invites,
        verifier,
        public_base_url,
    });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
