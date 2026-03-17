# Printing Press

Subscription and email backend for [philipithomas.com](https://philipithomas.com).

## Features

- Subscriber management (create, verify, update, unsubscribe)
- Email verification via 6-digit code + magic link
- Newsletter subscription preferences (Postcard, Contraption, Workshop)
- Bulk email sending via Postgres-backed queue + AWS SES
- CLI tool (`pp`) for triggering newsletter sends
- Token-based unsubscribe with per-newsletter preference management
- Local email preview via Mailpit
- OpenAPI documentation with Swagger UI

## Quick Start

```bash
# Start PostgreSQL + Mailpit
docker compose up -d

# Run the server (migrations run automatically, queue worker starts)
cargo run

# Health check
curl http://localhost:8080/health

# View API docs
open http://localhost:8080/docs

# View emails sent locally
open http://localhost:8025
```

## CLI (`pp`)

The `pp` CLI orchestrates newsletter sends. It fetches post content from the website, validates with the server, and enqueues emails for background delivery.

### Install

```bash
cargo install --path .
```

This installs both the `printing-press` server and the `pp` CLI.

### Authentication

- **Development**: Uses hardcoded API key (`dev-api-key`), no setup needed.
- **Production**: Retrieves API key from 1Password CLI (`op item get printing-press --field M2M_API_KEY`). Requires [1Password CLI](https://developer.1password.com/docs/cli/) to be installed and authenticated.

### Publish

```bash
# Test send to yourself
pp publish my-post --to your@email.com

# Production test send
pp -e prd publish my-post --to your@email.com

# Send to all subscribers
pp -e prd publish my-post

# Force send (if some subscribers already received it)
pp -e prd publish my-post --force
```

The publish command:
1. Fetches post metadata + email HTML from the website (`GET /api/posts/{slug}`)
2. Validates subscriber counts with the server
3. Prompts for confirmation before sending
4. Enqueues emails for background delivery via the queue worker

### Environments

| Name | Server | Website | API Key Source |
|------|--------|---------|---------------|
| `development` (default) | `http://localhost:8080` | `http://localhost:3000` | Hardcoded |
| `prd` / `production` | `https://printing-press.contraption.co` | `https://philipithomas.com` | 1Password CLI |

## Email Delivery

Emails are sent via a Postgres-backed queue with a background worker:

1. `pp publish` (or the `/api/v1/publish/send` endpoint) inserts rows into `email_sends` with `next_attempt_at = NOW()`
2. The queue worker polls every second, sends emails at the configured rate limit, and marks rows as sent
3. Failed sends retry with exponential backoff (30s, 60s, 120s, 240s, 480s), max 5 attempts
4. Outgoing newsletter emails include `List-Unsubscribe` and `List-Unsubscribe-Post` headers for native one-click unsubscribe in Gmail/Apple Mail

In development, emails are sent via SMTP to Mailpit (viewable at `http://localhost:8025`). In production, emails are sent via AWS SES.

## Unsubscribe

Each email contains a token-based unsubscribe link pointing to the frontend (`/unsubscribe?token=...`). The frontend proxies to these printing-press endpoints:

- `GET /api/v1/unsubscribe/{token}/preferences` — masked email + subscription state
- `PATCH /api/v1/unsubscribe/{token}/preferences` — toggle individual newsletters
- `DELETE /api/v1/unsubscribe/{token}/account` — permanently delete subscriber and all data

The legacy `GET /api/v1/unsubscribe/{token}` endpoint redirects to the frontend unsubscribe page.

## Configuration

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5433/printing_press` | PostgreSQL connection |
| `M2M_API_KEY` | `dev-api-key` | API authentication key |
| `AWS_REGION` | `us-east-1` | AWS region for SES |
| `SES_FROM_EMAIL` | `mail@philipithomas.com` | Sender email address |
| `SITE_URL` | `http://localhost:3000` | Frontend URL for unsubscribe links |
| `HOST` | `0.0.0.0` | Server bind address |
| `PORT` | `8080` | Server port |
| `EMAIL_BACKEND` | `smtp` | Email backend: `smtp` (Mailpit) or `ses` (AWS) |
| `SMTP_HOST` | `localhost` | SMTP host for local dev |
| `SMTP_PORT` | `1025` | SMTP port for local dev |
| `SES_RATE_PER_SECOND` | `14` | Max emails per second (SES rate limit) |

## API

All endpoints except `/health`, token-based unsubscribe, and unsubscribe preferences require `x-api-key` header.

See `/docs` for full API documentation.

## Testing

```bash
# Requires Docker for testcontainers
cargo test
```

## License

MIT - Philip I. Thomas
