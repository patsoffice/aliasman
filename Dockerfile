FROM rust:1.87-bookworm AS builder

RUN apt-get update && apt-get install -y \
    libsqlite3-dev \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release --package aliasman-cli --package aliasman-web

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libsqlite3-0 \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/aliasman /usr/local/bin/aliasman
COPY --from=builder /app/target/release/aliasman-web /usr/local/bin/aliasman-web

EXPOSE 3000

ENTRYPOINT ["aliasman-web", "--bind", "0.0.0.0:3000"]
