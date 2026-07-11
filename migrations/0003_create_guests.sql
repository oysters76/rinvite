CREATE TABLE IF NOT EXISTS guests (
    id             UUID PRIMARY KEY,
    event_id       UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    name           TEXT NOT NULL,
    channel        TEXT NOT NULL,
    max_party_size INTEGER NOT NULL,
    invite_token   TEXT NOT NULL UNIQUE,
    rsvp_status    TEXT NOT NULL,
    party_size     INTEGER,
    responded_at   TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_guests_event ON guests (event_id);
CREATE INDEX IF NOT EXISTS idx_guests_token ON guests (invite_token);
