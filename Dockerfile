# syntax=docker/dockerfile:1.6

# -------- Builder stage --------
FROM rust:1.83-bookworm AS builder

# Install native deps for rocksdb, TLS, and sccache
RUN apt-get update -y \
 && apt-get install -y --no-install-recommends \
      build-essential clang pkg-config cmake libclang-dev \
      libssl-dev \
      librocksdb-dev \
 && rm -rf /var/lib/apt/lists/*

# Install sccache for faster compilation
RUN cargo install sccache

WORKDIR /app

# Enable sccache
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/sccache
ENV SCCACHE_GHA_ENABLED=true

# Leverage Docker layer caching for dependencies
# 1) Copy manifests first
COPY Cargo.toml Cargo.lock ./
# 2) Create a dummy src to satisfy cargo build deps
RUN mkdir -p src \
 && echo "fn main() {}" > src/main.rs \
 && mkdir -p src/bin \
 && echo "fn main() {}" > src/bin/dummy.rs

# 3) Prebuild dependencies with sccache
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/sccache \
    cargo build --release

# 4) Now copy the full source and build the actual binary
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/sccache \
    cargo build --release

# -------- Runtime stage --------
FROM debian:bookworm-slim AS runtime

# Minimal runtime deps
RUN apt-get update -y \
 && apt-get install -y --no-install-recommends ca-certificates wget \
 && rm -rf /var/lib/apt/lists/* \
 && update-ca-certificates

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/relayx /usr/local/bin/relayx

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
