# Copyright 2020 The scaphandre authors
# Copyright 2026 pain459
# SPDX-License-Identifier: MIT

FROM rust:1.95 AS builder
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/gpu-power-exporter /usr/local/bin/gpu-power-exporter
EXPOSE 8082
ENTRYPOINT ["gpu-power-exporter"]
