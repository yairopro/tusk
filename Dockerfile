# 1. Builder stage
FROM rust:bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Add wasm target for frontend
RUN rustup target add wasm32-unknown-unknown

# Install cargo-binstall and prebuilt cargo-leptos binary
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall --no-confirm cargo-leptos

WORKDIR /app
COPY . .

# Build release artifacts (server binary + wasm + tailwind assets)
RUN cargo leptos build --release

# 2. Runner stage
FROM debian:bookworm-slim AS runner

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    openssl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy server binary, site assets, and Cargo.toml (needed for Leptos runtime config)
COPY --from=builder /app/target/release/tusk /app/tusk
COPY --from=builder /app/target/site /app/site
COPY --from=builder /app/Cargo.toml /app/Cargo.toml

ENV LEPTOS_SITE_ROOT="site"
ENV LEPTOS_ENV="PROD"
ENV PORT="3000"

EXPOSE 3000

CMD ["sh", "-c", "LEPTOS_SITE_ADDR=0.0.0.0:${PORT:-3000} /app/tusk"]
