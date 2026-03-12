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
- **Email**: AWS SES (production) or SMTP/Mailpit (local dev) via EmailSender trait
- **Queue**: Postgres-backed email queue with background tokio worker
- **Auth**: M2M API key (x-api-key header)
- **Config**: Figment with env var overrides
- **CLI**: `pp` binary for orchestrating newsletter sends

## Key paths

- `src/routes/` - API endpoint handlers
- `src/models/` - Database models and queries
- `src/services/` - Business logic (email_service, queue_worker, login, subscriber)
- `src/templates/` - Email HTML templates
- `src/cli/` - CLI tool (`pp` binary)
- `migrations/` - SQL migrations (run on startup)
- `tests/` - Integration tests with testcontainers

## Style

- `cargo fmt` for formatting
- `cargo clippy -- -D warnings` for linting
- All SQL queries use sqlx::query_as (not compile-time macros, to avoid needing DB at build time)
