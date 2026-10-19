# ---- Build stage ----
FROM rust:1.89-bookworm AS builder

WORKDIR /app

# Cache dependencies (dummy sources so cargo can resolve the package)
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs && \
    echo "fn main() {}" > benches/limiter_bench.rs && \
    cargo build --release --bin distributed-rate-limiter 2>/dev/null || true && \
    rm -rf src

# Real sources (benches stays as stub — not needed for the binary image)
COPY src ./src
RUN touch src/main.rs src/lib.rs && \
    cargo build --release --bin distributed-rate-limiter

# ---- Runtime stage ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/distributed-rate-limiter /usr/local/bin/distributed-rate-limiter

ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info \
    FAIL_OPEN=true

EXPOSE 8080

CMD ["distributed-rate-limiter"]