FROM rust:1.85-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates chromium && rm -rf /var/lib/apt/lists/*

ENV CHROMIUM_PATH=/usr/bin/chromium

COPY --from=builder /app/target/release/printing-press /usr/local/bin/printing-press

ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080

CMD ["printing-press"]
