# printing-press

Subscription and email backend for philipithomas.com.

## Development

```bash
docker compose up -d       # Start PostgreSQL + Mailpit
cargo run                  # Start server (runs migrations automatically, starts queue worker)
cargo test                 # Run tests (uses testcontainers, needs Docker)
```

Mailpit web UI at http://localhost:8025 for previewing emails sent locally.

## Architecture

- **Framework**: Axum with Tower middleware
- **Database**: PostgreSQL via sqlx with compile-time query checking
- **Email**: AWS SES (production) or SMTP/Mailpit (local dev) via enum dispatch in EmailService
- **Queue**: Postgres-backed email queue (`email_sends` table) with background tokio worker, rate limiting, and exponential backoff retries
- **Auth**: M2M API key (`x-api-key` header) for most endpoints; token-based auth for unsubscribe endpoints
- **Config**: Figment with env var overrides
- **CLI**: `press` binary for orchestrating newsletter sends (separate binary in `src/cli/`)

## Key paths

- `src/routes/` — API endpoint handlers (publish, unsubscribe, subscribers, emails)
- `src/models/` — Database models and queries (subscriber, email_send, login)
- `src/services/` — Business logic (email_service, queue_worker, login, subscriber)
- `src/templates/` — Email HTML templates (newsletter, confirmation)
- `src/cli/` — CLI tool (`press` binary): commands, client, keystore, config
- `migrations/` — SQL migrations (run on startup)
- `tests/` — Integration tests with testcontainers

## Style

- `cargo fmt` for formatting
- `cargo clippy -- -D warnings` for linting
- All SQL queries use sqlx::query_as (not compile-time macros, to avoid needing DB at build time)
- Newsletter subscriber queries use `NOT EXISTS` subqueries (not `NOT IN`) for performance

## Maintaining this file

When making major changes (new features, architectural changes, new endpoints, new conventions), update this CLAUDE.md and README.md to reflect them. This file is the primary onboarding document for future sessions; README.md is the primary onboarding document for humans.
