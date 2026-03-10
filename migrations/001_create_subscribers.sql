CREATE TABLE IF NOT EXISTS subscribers (
    id BIGSERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    confirmed_at TIMESTAMPTZ,
    subscribed_postcard BOOLEAN NOT NULL DEFAULT TRUE,
    subscribed_contraption BOOLEAN NOT NULL DEFAULT TRUE,
    subscribed_workshop BOOLEAN NOT NULL DEFAULT TRUE,
    source TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subscribers_email ON subscribers (email);
CREATE INDEX idx_subscribers_uuid ON subscribers (uuid);
