# syntax=docker/dockerfile:1.6

# -------- Builder stage --------
FROM rust:latest AS builder

# Update to nightly for edition2024 support
RUN rustup default nightly

# Install native deps for rocksdb and TLS
RUN apt-get update -y \
 && apt-get install -y --no-install-recommends \
      build-essential clang pkg-config cmake libclang-dev \
      libssl-dev \
      librocksdb-dev \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Leverage Docker layer caching for dependencies
# 1) Copy manifests first
COPY Cargo.toml ./
# Copy Cargo.lock if it exists, otherwise cargo will generate it
COPY Cargo.loc[k] ./
# 2) Create a dummy src to satisfy cargo build deps
RUN mkdir -p src \
 && echo "fn main() {}" > src/main.rs \
 && mkdir -p src/bin \
 && echo "fn main() {}" > src/bin/dummy.rs

# 3) Prebuild dependencies
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release

# 4) Now copy the full source and build the actual binary
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp /app/target/release/relayx /app/relayx

# -------- Runtime stage --------
FROM debian:bookworm-slim AS runtime

# Minimal runtime deps
RUN apt-get update -y \
 && apt-get install -y --no-install-recommends ca-certificates wget \
 && rm -rf /var/lib/apt/lists/* \
 && update-ca-certificates

WORKDIR /app

# Copy binary
COPY --from=builder /app/relayx /usr/local/bin/relayx

# Default configuration path; mount or bake your config.json
ENV RELAYX_CONFIG=/app/config.json \
    RUST_LOG=info

# Default HTTP bind settings (can be overridden)
ENV HTTP_ADDRESS=0.0.0.0 \
    HTTP_PORT=4937 \
    HTTP_CORS=*

EXPOSE 4937

# Healthcheck (optional)
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD wget -qO- http://127.0.0.1:${HTTP_PORT}/ || exit 1

# Entrypoint uses CLI flags that mirror envs; config path via RELAYX_CONFIG
ENTRYPOINT ["/usr/local/bin/relayx"]
CMD ["--http-address", "${HTTP_ADDRESS}", "--http-port", "${HTTP_PORT}", "--http-cors", "${HTTP_CORS}"]
