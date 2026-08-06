use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::{Event, Precedence};
use crate::domain::guest::{Guest, InviteChannel, RsvpStatus};
use crate::domain::port::outbound::{EventRepository, GuestRepository};

/// Postgres adapter backing both `EventRepository` and `GuestRepository`.
/// Plain SQL, no ORM — the same style as the user repository.
#[derive(Clone)]
pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn repo_err(e: impl std::fmt::Display) -> DomainError {
    DomainError::Repository(e.to_string())
}

fn row_to_event(row: &PgRow) -> Result<Event, DomainError> {
    Ok(Event {
        id: row.try_get("id").map_err(repo_err)?,
        owner_id: row.try_get("owner_id").map_err(repo_err)?,
        bride_name: row.try_get("bride_name").map_err(repo_err)?,
        bride_family_name: row.try_get("bride_family_name").map_err(repo_err)?,
        groom_name: row.try_get("groom_name").map_err(repo_err)?,
        groom_family_name: row.try_get("groom_family_name").map_err(repo_err)?,
        bride_phone: row.try_get("bride_phone").map_err(repo_err)?,
        groom_phone: row.try_get("groom_phone").map_err(repo_err)?,
        precedence: Precedence::parse(&row.try_get::<String, _>("precedence").map_err(repo_err)?),
        event_date: row
            .try_get::<NaiveDate, _>("event_date")
            .map_err(repo_err)?,
        start_time: row
            .try_get::<NaiveTime, _>("start_time")
            .map_err(repo_err)?,
        end_time: row.try_get::<NaiveTime, _>("end_time").map_err(repo_err)?,
        hall_name: row.try_get("hall_name").map_err(repo_err)?,
        venue_name: row.try_get("venue_name").map_err(repo_err)?,
        rsvp_by: row.try_get::<NaiveDate, _>("rsvp_by").map_err(repo_err)?,
        poruwa_ceremony_time: row
            .try_get::<Option<NaiveTime>, _>("poruwa_ceremony_time")
            .map_err(repo_err)?,
        created_at: row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(repo_err)?,
    })
}

fn row_to_guest(row: &PgRow) -> Result<Guest, DomainError> {
    let channel: String = row.try_get("channel").map_err(repo_err)?;
    let status: String = row.try_get("rsvp_status").map_err(repo_err)?;
    let max_party_size: i32 = row.try_get("max_party_size").map_err(repo_err)?;
    let party_size: Option<i32> = row.try_get("party_size").map_err(repo_err)?;
    Ok(Guest {
        id: row.try_get("id").map_err(repo_err)?,
        event_id: row.try_get("event_id").map_err(repo_err)?,
        name: row.try_get("name").map_err(repo_err)?,
        channel: InviteChannel::parse(&channel)?,
        email: row.try_get("email").map_err(repo_err)?,
        phone: row.try_get("phone").map_err(repo_err)?,
        max_party_size: max_party_size as u16,
        invite_token: row.try_get("invite_token").map_err(repo_err)?,
        rsvp_status: RsvpStatus::parse(&status)?,
        party_size: party_size.map(|p| p as u16),
        responded_at: row
            .try_get::<Option<DateTime<Utc>>, _>("responded_at")
            .map_err(repo_err)?,
        created_at: row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(repo_err)?,
    })
}

#[async_trait]
impl EventRepository for PostgresEventStore {
    async fn save(&self, e: &Event) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO events (id, owner_id, bride_name, bride_family_name, groom_name, \
             groom_family_name, bride_phone, groom_phone, precedence, event_date, start_time, \
             end_time, hall_name, venue_name, rsvp_by, poruwa_ceremony_time, created_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)",
        )
        .bind(e.id)
        .bind(e.owner_id)
        .bind(&e.bride_name)
        .bind(&e.bride_family_name)
        .bind(&e.groom_name)
        .bind(&e.groom_family_name)
        .bind(&e.bride_phone)
        .bind(&e.groom_phone)
        .bind(e.precedence.as_str())
        .bind(e.event_date)
        .bind(e.start_time)
        .bind(e.end_time)
        .bind(&e.hall_name)
        .bind(&e.venue_name)
        .bind(e.rsvp_by)
        .bind(e.poruwa_ceremony_time)
        .bind(e.created_at)
        .execute(&self.pool)
        .await
        .map_err(repo_err)?;
        Ok(())
    }

    async fn find(&self, id: Uuid) -> Result<Option<Event>, DomainError> {
        let row = sqlx::query("SELECT * FROM events WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(repo_err)?;
        row.as_ref().map(row_to_event).transpose()
    }

    async fn list_by_owner(&self, owner_id: Uuid) -> Result<Vec<Event>, DomainError> {
        let rows = sqlx::query("SELECT * FROM events WHERE owner_id = $1 ORDER BY created_at DESC")
            .bind(owner_id)
            .fetch_all(&self.pool)
            .await
            .map_err(repo_err)?;
        rows.iter().map(row_to_event).collect()
    }

    async fn update(&self, e: &Event) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE events SET bride_name=$2, bride_family_name=$3, groom_name=$4, \
             groom_family_name=$5, bride_phone=$6, groom_phone=$7, precedence=$8, event_date=$9, \
             start_time=$10, end_time=$11, hall_name=$12, venue_name=$13, rsvp_by=$14, \
             poruwa_ceremony_time=$15 WHERE id=$1",
        )
        .bind(e.id)
        .bind(&e.bride_name)
        .bind(&e.bride_family_name)
        .bind(&e.groom_name)
        .bind(&e.groom_family_name)
        .bind(&e.bride_phone)
        .bind(&e.groom_phone)
        .bind(e.precedence.as_str())
        .bind(e.event_date)
        .bind(e.start_time)
        .bind(e.end_time)
        .bind(&e.hall_name)
        .bind(&e.venue_name)
        .bind(e.rsvp_by)
        .bind(e.poruwa_ceremony_time)
        .execute(&self.pool)
        .await
        .map_err(repo_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        // guests.event_id has ON DELETE CASCADE, so its guests go with it.
        sqlx::query("DELETE FROM events WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(repo_err)?;
        Ok(())
    }
}

#[async_trait]
impl GuestRepository for PostgresEventStore {
    async fn save(&self, g: &Guest) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO guests (id, event_id, name, channel, email, phone, max_party_size, \
             invite_token, rsvp_status, party_size, responded_at, created_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(g.id)
        .bind(g.event_id)
        .bind(&g.name)
        .bind(g.channel.as_str())
        .bind(&g.email)
        .bind(&g.phone)
        .bind(g.max_party_size as i32)
        .bind(&g.invite_token)
        .bind(g.rsvp_status.as_str())
        .bind(g.party_size.map(|p| p as i32))
        .bind(g.responded_at)
        .bind(g.created_at)
        .execute(&self.pool)
        .await
        .map_err(repo_err)?;
        Ok(())
    }

    async fn save_many(&self, guests: &[Guest]) -> Result<(), DomainError> {
        if guests.is_empty() {
            return Ok(());
        }

        // One multi-row INSERT rather than a statement per guest: a bulk import
        // costs a single round trip instead of one per row, which is what made
        // large CSV imports time out. A single statement is atomic on its own,
        // so the all-or-nothing contract holds without an explicit transaction.
        //
        // UNNEST (twelve column arrays) rather than a generated VALUES list: the
        // SQL text is constant, so sqlx's prepared-statement cache hits whatever
        // the batch size is, and the parameter count stays at 12 — nowhere near
        // the 65535 ceiling a per-row VALUES form would approach.
        let mut ids = Vec::with_capacity(guests.len());
        let mut event_ids = Vec::with_capacity(guests.len());
        let mut names = Vec::with_capacity(guests.len());
        let mut channels = Vec::with_capacity(guests.len());
        let mut emails = Vec::with_capacity(guests.len());
        let mut phones = Vec::with_capacity(guests.len());
        let mut max_party_sizes = Vec::with_capacity(guests.len());
        let mut invite_tokens = Vec::with_capacity(guests.len());
        let mut rsvp_statuses = Vec::with_capacity(guests.len());
        let mut party_sizes = Vec::with_capacity(guests.len());
        let mut responded_ats = Vec::with_capacity(guests.len());
        let mut created_ats = Vec::with_capacity(guests.len());

        for g in guests {
            // Same conversions as `save` above — keep the two in step.
            ids.push(g.id);
            event_ids.push(g.event_id);
            names.push(g.name.clone());
            channels.push(g.channel.as_str().to_owned());
            emails.push(g.email.clone());
            phones.push(g.phone.clone());
            max_party_sizes.push(g.max_party_size as i32);
            invite_tokens.push(g.invite_token.clone());
            rsvp_statuses.push(g.rsvp_status.as_str().to_owned());
            party_sizes.push(g.party_size.map(|p| p as i32));
            responded_ats.push(g.responded_at);
            created_ats.push(g.created_at);
        }

        sqlx::query(
            "INSERT INTO guests (id, event_id, name, channel, email, phone, max_party_size, \
             invite_token, rsvp_status, party_size, responded_at, created_at) \
             SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::text[], $4::text[], $5::text[], \
             $6::text[], $7::int4[], $8::text[], $9::text[], $10::int4[], $11::timestamptz[], \
             $12::timestamptz[])",
        )
        .bind(&ids)
        .bind(&event_ids)
        .bind(&names)
        .bind(&channels)
        .bind(&emails)
        .bind(&phones)
        .bind(&max_party_sizes)
        .bind(&invite_tokens)
        .bind(&rsvp_statuses)
        .bind(&party_sizes)
        .bind(&responded_ats)
        .bind(&created_ats)
        .execute(&self.pool)
        .await
        .map_err(repo_err)?;
        Ok(())
    }

    async fn find(&self, id: Uuid) -> Result<Option<Guest>, DomainError> {
        let row = sqlx::query("SELECT * FROM guests WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(repo_err)?;
        row.as_ref().map(row_to_guest).transpose()
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<Guest>, DomainError> {
        let row = sqlx::query("SELECT * FROM guests WHERE invite_token = $1")
            .bind(token)
            .fetch_optional(&self.pool)
            .await
            .map_err(repo_err)?;
        row.as_ref().map(row_to_guest).transpose()
    }

    async fn list_by_event(&self, event_id: Uuid) -> Result<Vec<Guest>, DomainError> {
        let rows = sqlx::query("SELECT * FROM guests WHERE event_id = $1 ORDER BY created_at ASC")
            .bind(event_id)
            .fetch_all(&self.pool)
            .await
            .map_err(repo_err)?;
        rows.iter().map(row_to_guest).collect()
    }

    async fn update_rsvp(&self, g: &Guest) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE guests SET rsvp_status = $1, party_size = $2, responded_at = $3 WHERE id = $4",
        )
        .bind(g.rsvp_status.as_str())
        .bind(g.party_size.map(|p| p as i32))
        .bind(g.responded_at)
        .bind(g.id)
        .execute(&self.pool)
        .await
        .map_err(repo_err)?;
        Ok(())
    }

    async fn update(&self, g: &Guest) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE guests SET name=$2, channel=$3, email=$4, phone=$5, max_party_size=$6 \
             WHERE id=$1",
        )
        .bind(g.id)
        .bind(&g.name)
        .bind(g.channel.as_str())
        .bind(&g.email)
        .bind(&g.phone)
        .bind(g.max_party_size as i32)
        .execute(&self.pool)
        .await
        .map_err(repo_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM guests WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(repo_err)?;
        Ok(())
    }
}

/// Tests that need a real Postgres. They are `#[ignore]`d so `cargo test` stays
/// database-free (CI has no database); run them against the docker-compose
/// instance with:
///
/// ```text
/// DATABASE_URL=postgres://... cargo test -- --ignored
/// ```
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    /// Connect and migrate, or skip the test when no database is configured.
    async fn pool_or_skip() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("connect to DATABASE_URL");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        Some(pool)
    }

    /// An owner row and an event to hang the guests off — both FKs are enforced.
    async fn seed_event(pool: &PgPool) -> (Uuid, Uuid) {
        let owner_id = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1,$2,$3)")
            .bind(owner_id)
            .bind(format!("save-many-{owner_id}@example.test"))
            .bind("x")
            .execute(pool)
            .await
            .expect("insert owner");

        let event = Event {
            id: Uuid::new_v4(),
            owner_id,
            bride_name: "Ann".to_owned(),
            bride_family_name: "Silva".to_owned(),
            groom_name: "Bo".to_owned(),
            groom_family_name: "Perera".to_owned(),
            bride_phone: "+94711111111".to_owned(),
            groom_phone: "+94722222222".to_owned(),
            precedence: Precedence::Bride,
            event_date: NaiveDate::from_ymd_opt(2030, 6, 1).unwrap(),
            start_time: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
            hall_name: "Ballroom".to_owned(),
            venue_name: "Rest House".to_owned(),
            rsvp_by: NaiveDate::from_ymd_opt(2030, 5, 1).unwrap(),
            poruwa_ceremony_time: None,
            created_at: Utc::now(),
        };
        let store = PostgresEventStore::new(pool.clone());
        EventRepository::save(&store, &event)
            .await
            .expect("save event");
        (owner_id, event.id)
    }

    /// Deleting the event cascades to its guests; the owner goes last.
    async fn cleanup(pool: &PgPool, owner_id: Uuid, event_id: Uuid) {
        sqlx::query("DELETE FROM events WHERE id = $1")
            .bind(event_id)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(owner_id)
            .execute(pool)
            .await
            .ok();
    }

    /// The batched UNNEST insert is the risky part of this adapter: twelve array
    /// binds, twelve casts, and a column order that has to line up. Insert a
    /// batch larger than MAX_BULK covering both channels, both RSVP states, and
    /// NULL/non-NULL on every nullable column, then read it all back.
    #[tokio::test]
    #[ignore = "requires a Postgres at DATABASE_URL"]
    async fn save_many_round_trips_every_field() {
        let Some(pool) = pool_or_skip().await else {
            eprintln!("DATABASE_URL unset — skipping");
            return;
        };
        let (owner_id, event_id) = seed_event(&pool).await;
        let store = PostgresEventStore::new(pool.clone());

        let responded = Utc::now();
        let guests: Vec<Guest> = (0..600)
            .map(|i| {
                let answered = i % 2 == 0;
                Guest {
                    id: Uuid::new_v4(),
                    event_id,
                    name: format!("Guest {i}"),
                    // Alternate the enums so neither variant goes unexercised.
                    channel: if answered {
                        InviteChannel::EInvite
                    } else {
                        InviteChannel::Print
                    },
                    // Every nullable column is NULL on half the rows.
                    email: answered.then(|| format!("g{i}@example.test")),
                    phone: answered.then(|| format!("+9477000{i:04}")),
                    max_party_size: (i % 5) as u16 + 1,
                    invite_token: format!("tok-{}", Uuid::new_v4()),
                    rsvp_status: if answered {
                        RsvpStatus::Attending
                    } else {
                        RsvpStatus::Pending
                    },
                    party_size: answered.then_some((i % 3) as u16 + 1),
                    responded_at: answered.then_some(responded),
                    created_at: Utc::now(),
                }
            })
            .collect();

        GuestRepository::save_many(&store, &guests)
            .await
            .expect("save_many");

        let mut stored = store.list_by_event(event_id).await.expect("list_by_event");
        assert_eq!(stored.len(), guests.len());

        // list_by_event orders by created_at, which is not unique enough here.
        stored.sort_by_key(|g| g.id);
        let mut expected = guests.clone();
        expected.sort_by_key(|g| g.id);
        for (got, want) in stored.iter().zip(expected.iter()) {
            assert_eq!(got.id, want.id);
            assert_eq!(got.event_id, want.event_id);
            assert_eq!(got.name, want.name);
            assert_eq!(got.channel, want.channel);
            assert_eq!(got.email, want.email);
            assert_eq!(got.phone, want.phone);
            assert_eq!(got.max_party_size, want.max_party_size);
            assert_eq!(got.invite_token, want.invite_token);
            assert_eq!(got.rsvp_status, want.rsvp_status);
            assert_eq!(got.party_size, want.party_size);
            assert_eq!(got.responded_at, want.responded_at);
        }

        cleanup(&pool, owner_id, event_id).await;
    }

    /// A duplicate invite_token violates the UNIQUE index. Because the batch is
    /// a single statement, the failure must leave no rows behind at all.
    #[tokio::test]
    #[ignore = "requires a Postgres at DATABASE_URL"]
    async fn save_many_is_all_or_nothing() {
        let Some(pool) = pool_or_skip().await else {
            eprintln!("DATABASE_URL unset — skipping");
            return;
        };
        let (owner_id, event_id) = seed_event(&pool).await;
        let store = PostgresEventStore::new(pool.clone());

        let dup = format!("dup-{}", Uuid::new_v4());
        let guests: Vec<Guest> = (0..3)
            .map(|i| Guest {
                id: Uuid::new_v4(),
                event_id,
                name: format!("Guest {i}"),
                channel: InviteChannel::Print,
                email: None,
                phone: None,
                max_party_size: 1,
                // Rows 0 and 2 collide on the UNIQUE invite_token index.
                invite_token: if i == 1 {
                    format!("uniq-{}", Uuid::new_v4())
                } else {
                    dup.clone()
                },
                rsvp_status: RsvpStatus::Pending,
                party_size: None,
                responded_at: None,
                created_at: Utc::now(),
            })
            .collect();

        assert!(GuestRepository::save_many(&store, &guests).await.is_err());
        assert!(
            store
                .list_by_event(event_id)
                .await
                .expect("list_by_event")
                .is_empty(),
            "a failed batch must not persist any rows"
        );

        cleanup(&pool, owner_id, event_id).await;
    }

    /// An empty batch short-circuits without touching the database.
    #[tokio::test]
    #[ignore = "requires a Postgres at DATABASE_URL"]
    async fn save_many_accepts_an_empty_batch() {
        let Some(pool) = pool_or_skip().await else {
            eprintln!("DATABASE_URL unset — skipping");
            return;
        };
        let store = PostgresEventStore::new(pool);
        GuestRepository::save_many(&store, &[])
            .await
            .expect("empty batch is a no-op");
    }
}
