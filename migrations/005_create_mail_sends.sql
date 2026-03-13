CREATE TABLE mail_sends (
    id BIGSERIAL PRIMARY KEY,
    stripe_customer_id TEXT NOT NULL,
    customer_email TEXT,
    post_slug TEXT NOT NULL,
    newsletter TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    lob_letter_id TEXT,
    pages INTEGER,
    double_sided BOOLEAN NOT NULL DEFAULT FALSE,
    carrier TEXT,
    send_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at TIMESTAMPTZ
);

CREATE INDEX idx_mail_sends_idempotency ON mail_sends(idempotency_key);
CREATE INDEX idx_mail_sends_post_slug ON mail_sends(post_slug);
