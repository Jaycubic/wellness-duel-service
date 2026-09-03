-- Wellness Streak Duel — initial schema
-- UUIDs are generated application-side (uuid::Uuid::new_v4()), so no
-- pgcrypto / uuid-ossp extension is required on the server.

CREATE SCHEMA IF NOT EXISTS app;

CREATE TABLE IF NOT EXISTS app.rooms (
    id          UUID PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE,
    max_days    INTEGER NOT NULL DEFAULT 7,
    win_target  INTEGER NOT NULL DEFAULT 15,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS app.players (
    id                 UUID PRIMARY KEY,
    room_id            UUID NOT NULL REFERENCES app.rooms(id) ON DELETE CASCADE,
    device_token       TEXT NOT NULL,
    name               TEXT NOT NULL,
    streak             INTEGER NOT NULL DEFAULT 0,
    total_points       INTEGER NOT NULL DEFAULT 0,
    last_activity_key  TEXT,
    repeat_count       INTEGER NOT NULL DEFAULT 0,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (room_id, device_token)
);

CREATE TABLE IF NOT EXISTS app.checkins (
    id            UUID PRIMARY KEY,
    player_id     UUID NOT NULL REFERENCES app.players(id) ON DELETE CASCADE,
    day           INTEGER NOT NULL,
    activity_key  TEXT,
    points        INTEGER NOT NULL DEFAULT 0,
    skipped       BOOLEAN NOT NULL DEFAULT false,
    photo_path    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (player_id, day)
);

CREATE INDEX IF NOT EXISTS idx_players_room ON app.players(room_id);
CREATE INDEX IF NOT EXISTS idx_checkins_player ON app.checkins(player_id);
