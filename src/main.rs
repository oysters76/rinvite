mod adapter;
mod application;
mod domain;

use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use adapter::inbound::http::{AppState, html, routes};
use adapter::outbound::argon2_hasher::Argon2Hasher;
use adapter::outbound::clock::SystemClock;
use adapter::outbound::dispatch_sender::DispatchSender;
use adapter::outbound::email::log::LogEmailClient;
use adapter::outbound::email::resend::ResendClient;
use adapter::outbound::jwt_issuer::JwtIssuer;
use adapter::outbound::message::MessageTemplates;
use adapter::outbound::message::account::AccountTemplates;
use adapter::outbound::pdf::TemplatePdfRenderer;
use adapter::outbound::persistence::events_memory::InMemoryEventStore;
use adapter::outbound::persistence::events_postgres::PostgresEventStore;
use adapter::outbound::persistence::memory::InMemoryUserRepository;
use adapter::outbound::persistence::postgres::PostgresUserRepository;
use adapter::outbound::whatsapp::log::LogWhatsAppClient;
use adapter::outbound::whatsapp::twilio::TwilioWhatsApp;
use application::auth_service::AuthServiceImpl;
use application::billing_service::BillingServiceImpl;
use application::event_service::EventServiceImpl;
use application::invite_service::InviteServiceImpl;
use domain::port::inbound::{AuthService, BillingService, EventService, InviteService};
use domain::port::outbound::{
    Clock, EmailClient, EventRepository, GuestRepository, InvitePdfRenderer, InviteSender,
    PasswordHasher, TokenIssuer, TokenVerifier, UserRepository, WhatsAppClient,
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
    // Business contact shown to users in the "limit reached" dialog, and where
    // upgrade-request notifications are delivered (defaults to the contact).
    let contact_email = std::env::var("BUSINESS_CONTACT_EMAIL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "hello@example.com".to_owned());
    let upgrade_notify_email = std::env::var("UPGRADE_NOTIFY_EMAIL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| contact_email.clone());

    // ===== Outbound adapters (driven side) ==================================
    let hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2Hasher);
    let jwt = Arc::new(JwtIssuer::new(jwt_secret, token_ttl_secs));
    let tokens: Arc<dyn TokenIssuer> = jwt.clone();
    let verifier: Arc<dyn TokenVerifier> = jwt.clone();
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let pdf: Arc<dyn InvitePdfRenderer> = Arc::new(TemplatePdfRenderer::from_env()?);
    // Delivery: real Resend/Twilio clients when their keys are set, else keyless
    // Log* clients that print the message (with the pretty link) — so Send works
    // in local dev without any provider account.
    let email: Arc<dyn EmailClient> =
        match (std::env::var("RESEND_API_KEY"), std::env::var("EMAIL_FROM")) {
            (Ok(key), Ok(from)) if !key.is_empty() => Arc::new(ResendClient::new(key, from)),
            _ => {
                eprintln!(
                    "[startup] RESEND_API_KEY/EMAIL_FROM unset — emails will be logged, not sent"
                );
                Arc::new(LogEmailClient)
            }
        };
    let whatsapp: Arc<dyn WhatsAppClient> = match (
        std::env::var("TWILIO_ACCOUNT_SID"),
        std::env::var("TWILIO_AUTH_TOKEN"),
        std::env::var("TWILIO_WHATSAPP_FROM"),
    ) {
        (Ok(sid), Ok(token), Ok(from)) if !sid.is_empty() => Arc::new(TwilioWhatsApp::new(
            sid,
            token,
            from,
            std::env::var("TWILIO_CONTENT_SID")
                .ok()
                .filter(|s| !s.is_empty()),
        )),
        _ => {
            eprintln!("[startup] TWILIO_* unset — WhatsApp messages will be logged, not sent");
            Arc::new(LogWhatsAppClient)
        }
    };
    let sender: Arc<dyn InviteSender> = Arc::new(DispatchSender::new(
        email.clone(),
        whatsapp,
        MessageTemplates::from_env()?,
    ));

    // Account-lifecycle email templates (verification + upgrade-request).
    let account_templates = AccountTemplates::from_env()?;

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
    let auth: Arc<dyn AuthService> = Arc::new(AuthServiceImpl::new(
        users.clone(),
        hasher,
        tokens,
        email.clone(),
        clock.clone(),
        account_templates.clone(),
        public_base_url.clone(),
    ));
    let events: Arc<dyn EventService> = Arc::new(EventServiceImpl::new(
        events_repo.clone(),
        guests_repo.clone(),
        users.clone(),
        pdf,
        sender,
        clock.clone(),
        public_base_url.clone(),
    ));
    let invites: Arc<dyn InviteService> =
        Arc::new(InviteServiceImpl::new(events_repo, guests_repo, clock.clone()));
    let billing: Arc<dyn BillingService> = Arc::new(BillingServiceImpl::new(
        users,
        email,
        clock,
        account_templates,
        upgrade_notify_email,
    ));

    // ===== Inbound adapter (HTTP) + serve ===================================
    let einvite_template = html::load_template()?;

    let app = routes(AppState {
        auth,
        events,
        invites,
        billing,
        verifier,
        public_base_url,
        contact_email,
        einvite_template,
    });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
