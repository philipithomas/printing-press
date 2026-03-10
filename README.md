# Printing Press

Subscription and email backend for [philipithomas.com](https://philipithomas.com).

## Features

- Subscriber management (create, verify, update, unsubscribe)
- Email verification via 6-digit code + magic link
- Newsletter subscription preferences (Postcard, Contraption, Workshop)
- Email sending via AWS SES
- OpenAPI documentation with Swagger UI

## Quick Start

```bash
# Start PostgreSQL
docker compose up -d

# Run the server (migrations run automatically)
cargo run

# Health check
curl http://localhost:8080/health

# View API docs
open http://localhost:8080/docs
```

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

## API

All endpoints except `/health` and token-based unsubscribe require `x-api-key` header.

See `/docs` for full API documentation.

## Testing

```bash
# Requires Docker for testcontainers
cargo test
```

## License

MIT - Philip I. Thomas
