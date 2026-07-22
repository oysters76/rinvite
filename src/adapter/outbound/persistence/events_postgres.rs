use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::Event;
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
             groom_family_name, event_date, start_time, end_time, hall_name, venue_name, \
             rsvp_by, poruwa_ceremony_time, created_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        )
        .bind(e.id)
        .bind(e.owner_id)
        .bind(&e.bride_name)
        .bind(&e.bride_family_name)
        .bind(&e.groom_name)
        .bind(&e.groom_family_name)
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
             groom_family_name=$5, event_date=$6, start_time=$7, end_time=$8, hall_name=$9, \
             venue_name=$10, rsvp_by=$11, poruwa_ceremony_time=$12 WHERE id=$1",
        )
        .bind(e.id)
        .bind(&e.bride_name)
        .bind(&e.bride_family_name)
        .bind(&e.groom_name)
        .bind(&e.groom_family_name)
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
        // One transaction so a failure part-way rolls the whole batch back —
        // callers rely on all-or-nothing semantics.
        let mut tx = self.pool.begin().await.map_err(repo_err)?;
        for g in guests {
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
            .execute(&mut *tx)
            .await
            .map_err(repo_err)?;
        }
        tx.commit().await.map_err(repo_err)?;
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
