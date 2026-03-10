CREATE TABLE IF NOT EXISTS email_sends (
    id BIGSERIAL PRIMARY KEY,
    subscriber_id BIGINT NOT NULL REFERENCES subscribers(id),
    post_slug TEXT NOT NULL,
    unsubscribe_token UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    send_error TEXT,
    triggered_unsubscribe_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_email_sends_subscriber ON email_sends (subscriber_id);
CREATE INDEX idx_email_sends_unsubscribe_token ON email_sends (unsubscribe_token);
