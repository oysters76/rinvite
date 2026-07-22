-- Second verification gate: after email verification, the app owner must
-- manually approve an account before it can log in. `created_at` gives the
-- stale-account cleanup sweep a timestamp to measure the grace period against.
ALTER TABLE users ADD COLUMN IF NOT EXISTS approval_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE users ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Backfill: existing accounts predate this feature — approve them so current
-- users are not locked out. New signups insert 'pending' explicitly.
UPDATE users SET approval_status = 'approved';

CREATE INDEX IF NOT EXISTS idx_users_approval_status ON users (approval_status);
