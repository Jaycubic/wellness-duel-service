-- In-room chat messages

CREATE TABLE IF NOT EXISTS app.messages (
    id          UUID PRIMARY KEY,
    room_id     UUID NOT NULL REFERENCES app.rooms(id) ON DELETE CASCADE,
    player_id   UUID NOT NULL REFERENCES app.players(id) ON DELETE CASCADE,
    sender_name TEXT NOT NULL,
    body        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_messages_room ON app.messages(room_id, created_at DESC);
