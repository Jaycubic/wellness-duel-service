-- Feedback — public reviews with star ratings

CREATE TABLE IF NOT EXISTS app.feedback (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    rating      INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    message     TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_feedback_created ON app.feedback(created_at DESC);
