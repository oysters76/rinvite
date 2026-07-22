use std::sync::Arc;
use std::time::Duration;

use crate::domain::port::outbound::{Clock, UserRepository};

/// Spawn an in-process background task that periodically deletes stale accounts —
/// those created more than `retention` ago that never completed both verification
/// stages (email unverified, or verified but not owner-approved).
///
/// A sweep runs once immediately at startup, then every `interval`. Errors are
/// logged and swallowed so a transient DB hiccup never kills the task.
pub fn spawn_stale_account_cleanup(
    users: Arc<dyn UserRepository>,
    clock: Arc<dyn Clock>,
    retention: chrono::Duration,
    interval: Duration,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let cutoff = clock.now() - retention;
            match users.delete_stale(cutoff).await {
                Ok(0) => {}
                Ok(n) => println!("[cleanup] removed {n} stale unapproved account(s)"),
                Err(e) => eprintln!("[cleanup] sweep failed: {e}"),
            }
        }
    });
}
