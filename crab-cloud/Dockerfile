# ── Stage 1: Build ──
FROM rust:1.91-bookworm AS builder

# Install build dependencies (OpenSSL, aws-lc-rs needs cmake + clang)
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev cmake clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace manifests — only crab-cloud and its dependencies
COPY Cargo.toml Cargo.lock ./
COPY shared/Cargo.toml shared/Cargo.toml
COPY crab-cert/Cargo.toml crab-cert/Cargo.toml
COPY crab-cloud/Cargo.toml crab-cloud/Cargo.toml

# Trim workspace to only needed members
RUN printf '[workspace]\nresolver = "2"\nmembers = ["shared", "crab-cert", "crab-cloud"]\n' > /tmp/ws.toml && \
    sed -n '/^\[workspace\.package\]/,$ p' Cargo.toml >> /tmp/ws.toml && \
    mv /tmp/ws.toml Cargo.toml

# Create dummy source files to build dependencies
RUN mkdir -p shared/src crab-cert/src crab-cloud/src && \
    echo "pub fn _dummy() {}" > shared/src/lib.rs && \
    echo "pub fn _dummy() {}" > crab-cert/src/lib.rs && \
    echo "fn main() {}" > crab-cloud/src/main.rs

# Pre-build dependencies (cached unless Cargo.toml/Cargo.lock change)
RUN cargo build --release -p crab-cloud 2>/dev/null || true

# Copy real source code
COPY shared/ shared/
COPY crab-cert/ crab-cert/
COPY crab-cloud/ crab-cloud/

# Touch source files to invalidate the dummy build
RUN touch shared/src/lib.rs crab-cert/src/lib.rs crab-cloud/src/main.rs

# Build the real binary (inject git hash for /health endpoint)
ARG GIT_HASH=dev
ENV GIT_HASH=${GIT_HASH}
RUN cargo build --release -p crab-cloud

# ── Stage 2: Runtime ──
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false crab

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/crab-cloud .

# Copy migrations
COPY --from=builder /app/crab-cloud/migrations ./migrations

# Switch to non-root user
USER crab

# Default ports: 8080 (HTTP), 8443 (mTLS)
EXPOSE 8080 8443

ENV RUST_LOG=crab_cloud=info,tower_http=info

ENTRYPOINT ["./crab-cloud"]
