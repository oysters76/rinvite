CREATE TABLE IF NOT EXISTS events (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL REFERENCES users(id),
    bride_name        TEXT NOT NULL,
    bride_family_name TEXT NOT NULL,
    groom_name        TEXT NOT NULL,
    groom_family_name TEXT NOT NULL,
    event_date        DATE NOT NULL,
    start_time        TIME NOT NULL,
    end_time          TIME NOT NULL,
    hall_name         TEXT NOT NULL,
    venue_name        TEXT NOT NULL,
    rsvp_by           DATE NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_owner ON events (owner_id);
