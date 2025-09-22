# syntax=docker/dockerfile:1.6

# -------- Builder stage --------
FROM rust:1.83-bookworm AS builder

# Install native deps for rocksdb and TLS
RUN apt-get update -y \
 && apt-get install -y --no-install-recommends \
      build-essential clang pkg-config cmake libclang-dev \
      libssl-dev \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Leverage Docker layer caching for dependencies
# 1) Copy manifests first
COPY Cargo.toml Cargo.lock ./
# 2) Create a dummy src to satisfy cargo build deps
RUN mkdir -p src \
 && echo "fn main() {}" > src/main.rs \
 && mkdir -p src/bin \
 && echo "fn main() {}" > src/bin/dummy.rs

# 3) Prebuild dependencies (no default features or with optional onchain)
ARG FEATURES=""
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    if [ -n "$FEATURES" ]; then \
      cargo build --release --features "$FEATURES"; \
    else \
      cargo build --release; \
    fi

# 4) Now copy the full source and build the actual binary
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    if [ -n "$FEATURES" ]; then \
      cargo build --release --features "$FEATURES"; \
    else \
      cargo build --release; \
    fi

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
# Example to enable on-chain: build with --build-arg FEATURES=onchain
ENTRYPOINT ["/usr/local/bin/relayx"]
CMD ["--http-address", "${HTTP_ADDRESS}", "--http-port", "${HTTP_PORT}", "--http-cors", "${HTTP_CORS}"]
