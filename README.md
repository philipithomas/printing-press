# Printing Press

Subscription and email backend for [philipithomas.com](https://philipithomas.com).

## Features

- Subscriber management (create, verify, update, unsubscribe)
- Email verification via 6-digit code + magic link
- Newsletter subscription preferences (Postcard, Contraption, Workshop)
- Bulk email sending via Postgres-backed queue + AWS SES
- CLI tool (`pp`) for triggering newsletter sends
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

### Install

```bash
cargo install --path .
```

This installs both the `printing-press` server and the `pp` CLI.

### Login

Store an encrypted API key for an environment:

```bash
# Development (default)
pp login

# Production
pp -e prd login
```

Keys are encrypted with a password and stored in `~/.printing-press/`.

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
1. Fetches post metadata from the website
2. Validates subscriber counts with the server
3. Prompts for confirmation before sending
4. Enqueues emails for background delivery

## Configuration

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5433/printing_press` | PostgreSQL connection |
| `M2M_API_KEY` | `dev-api-key` | API authentication key |
| `AWS_REGION` | `us-east-1` | AWS region for SES |
| `SES_FROM_EMAIL` | `mail@philipithomas.com` | Sender email address |
| `SITE_URL` | `http://localhost:3000` | Frontend URL for redirects |
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
