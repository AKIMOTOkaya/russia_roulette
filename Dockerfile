# ==========================================
# Stage 1: Build binary from Rust source
# ==========================================
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /usr/src/app

# Copy workspace configuration and dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps ./apps
COPY clients ./clients

# Build optimized release binary for roulette-server
RUN cargo build --release --bin roulette-server

# ==========================================
# Stage 2: Minimal Distroless runtime
# ==========================================
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/roulette-server /app/runtime/roulette-backend

ENV ROULETTE_ENV=production \
    ROULETTE_SERVER_MODE=public \
    ROULETTE_HTTP_HOST=0.0.0.0 \
    ROULETTE_HTTP_PORT=8080 \
    ROULETTE_RUNTIME_DIR=/app/runtime

EXPOSE 8080

ENTRYPOINT ["/app/runtime/roulette-backend"]
