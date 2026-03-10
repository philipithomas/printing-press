# printing-press

Subscription and email backend for philipithomas.com.

## Development

```bash
docker compose up -d       # Start PostgreSQL
cargo run                  # Start server (runs migrations automatically)
cargo test                 # Run tests (uses testcontainers, needs Docker)
```

## Architecture

- **Framework**: Axum with Tower middleware
- **Database**: PostgreSQL via sqlx with compile-time query checking
- **Email**: AWS SES with minijinja templates
- **Auth**: M2M API key (x-api-key header)
- **Config**: Figment with env var overrides

## Key paths

- `src/routes/` - API endpoint handlers
- `src/models/` - Database models and queries
- `src/services/` - Business logic
- `src/templates/` - Email HTML templates
- `migrations/` - SQL migrations (run on startup)
- `tests/` - Integration tests with testcontainers

## Style

- `cargo fmt` for formatting
- `cargo clippy -- -D warnings` for linting
- All SQL queries use sqlx::query_as (not compile-time macros, to avoid needing DB at build time)
