-- Whose side is listed first on the invite. Existing events default to 'bride'
-- so their invitations render exactly as before.
ALTER TABLE events ADD COLUMN IF NOT EXISTS precedence TEXT NOT NULL DEFAULT 'bride'
    CHECK (precedence IN ('bride', 'groom'));
