ALTER TABLE email_sends
  ADD COLUMN subject TEXT,
  ADD COLUMN html_content TEXT,
  ADD COLUMN newsletter TEXT,
  ADD COLUMN sent_at TIMESTAMPTZ,
  ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0,
  ADD COLUMN next_attempt_at TIMESTAMPTZ;

CREATE INDEX idx_email_sends_queue
  ON email_sends (next_attempt_at)
  WHERE sent_at IS NULL AND send_error IS NULL;

CREATE INDEX idx_email_sends_post_slug ON email_sends (post_slug);
