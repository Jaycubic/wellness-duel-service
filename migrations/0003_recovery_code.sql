-- Add personal recovery codes to players for cross-device identity recovery.

ALTER TABLE app.players ADD COLUMN IF NOT EXISTS recovery_code TEXT UNIQUE;

CREATE UNIQUE INDEX IF NOT EXISTS idx_players_recovery ON app.players(recovery_code);
