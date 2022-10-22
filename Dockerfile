FROM rust:slim as BUILD

RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential pkg-config libssl-dev libpq-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app/

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target/   \
    cargo build --release && \
    mv target/release/sponsorblock-mirror .

FROM debian:stable-slim

RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        libssl1.1 libpq5 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app/

COPY --from=BUILD /app/sponsorblock-mirror .
COPY --from=BUILD /app/Rocket.toml .

EXPOSE 8000

CMD ["/app/sponsorblock-mirror"]
