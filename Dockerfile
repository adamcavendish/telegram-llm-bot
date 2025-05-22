FROM rust:1.87-slim AS builder

WORKDIR /app
COPY . .
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
    && \
    rm -rf /var/lib/apt/lists/*
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
    && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/telegram-bot-llm /usr/local/bin/

RUN adduser --disabled-password --gecos "" appuser
USER appuser

ENV RUST_LOG="info"
CMD ["telegram-bot-llm"]