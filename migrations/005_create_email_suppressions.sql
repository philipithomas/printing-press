CREATE TABLE IF NOT EXISTS email_suppressions (
    id BIGSERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    reason TEXT NOT NULL,
    source TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_email_suppressions_email ON email_suppressions (email);
