CREATE TABLE IF NOT EXISTS logins (
    id BIGSERIAL PRIMARY KEY,
    subscriber_id BIGINT NOT NULL REFERENCES subscribers(id),
    token TEXT NOT NULL UNIQUE,
    token_type TEXT NOT NULL CHECK (token_type IN ('code', 'magic_link')),
    email_sent_at TIMESTAMPTZ,
    verified_at TIMESTAMPTZ,
    expired_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_logins_token ON logins (token);
CREATE INDEX idx_logins_subscriber ON logins (subscriber_id);
