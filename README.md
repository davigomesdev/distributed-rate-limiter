# Distributed Rate Limiter

[![CI](https://github.com/YOUR_USERNAME/distributed-rate-limiter/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/distributed-rate-limiter/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

A production-oriented **distributed rate limiter** written in Rust. It implements **Token Bucket** and **Sliding Window** algorithms backed by **Redis** (with an in-memory storage for tests and local use), exposes an **Actix Web** middleware and HTTP API, and ships with **Prometheus** metrics.

Designed as a clear, well-structured portfolio project: typed APIs, fail-open/fail-closed behavior, pluggable storage, and first-class observability.

---

## Features

| Feature | Description |
|--------|-------------|
| **Token Bucket** | Capacity + continuous refill rate; cost-based consumption |
| **Sliding Window** | Fixed limit over a rolling time window (Redis sorted sets) |
| **Redis storage** | Shared state across instances via Lua scripts (atomic) |
| **In-memory storage** | Fast local backend for unit tests and single-node demos |
| **Actix middleware** | Drop-in protection for routes; rate-limit response headers |
| **HTTP check API** | `POST /v1/check` for programmatic decisions |
| **Prometheus metrics** | Latency histogram, decision counters, backend error counters |
| **Fail-open / fail-closed** | Configurable behavior when Redis is unavailable |
| **Key prefix** | Constant `KEY_PREFIX` (`"rl"`) to namespace Redis keys |

---

## Architecture

```
                    ┌─────────────────────────────────────┐
                    │           Actix Web App             │
                    │  /health  /metrics  /v1/check  /api │
                    └──────────────┬──────────────────────┘
                                   │
                    ┌──────────────▼──────────────────────┐
                    │     RateLimitMiddleware (optional)  │
                    │     key extractor → RateLimiter     │
                    └──────────────┬──────────────────────┘
                                   │
                    ┌──────────────▼──────────────────────┐
                    │            RateLimiter              │
                    │  algorithm + key_prefix + metrics   │
                    │  fail_open / fail_closed            │
                    └──────────────┬──────────────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              ▼                                         ▼
     ┌─────────────────┐                     ┌─────────────────┐
     │  RedisStorage   │                     │     Memory      │
     │  (Lua scripts)  │                     │  (tests / local)│
     └─────────────────┘                     └─────────────────┘
```

**Request path (middleware):**

1. Extract a rate-limit key (default: client IP).
2. Call `RateLimiter::check(key, cost)`.
3. If denied → `429 Too Many Requests` with `Retry-After` and `X-RateLimit-*` headers.
4. If allowed → forward the request and attach limit headers on the response.
5. On storage failure → allow (fail-open) or return `503` (fail-closed).

---

## Quick start

### Prerequisites

- Rust toolchain (1.70+ recommended for edition 2024)
- Redis (local or via Docker)

### Run with Docker Compose

```bash
docker compose up --build
```

Services:

- **API**: `http://localhost:8080`
- **Redis**: `localhost:6379`

### Run locally

```bash
# Start Redis
docker run -d --name redis -p 6379:6379 redis:7-alpine

# Copy env and start the server
cp .env.example .env
cargo run --release
```

---

## HTTP API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Redis connectivity check |
| `GET` | `/metrics` | Prometheus text exposition |
| `POST` | `/v1/check` | Rate-limit decision for a key |
| `GET` | `/api/hello` | Example route protected by the middleware |

### `POST /v1/check`

Request:

```json
{
  "key": "user:42",
  "cost": 1
}
```

Response (`200` allowed / `429` denied):

```json
{
  "allowed": true,
  "limit": 100,
  "remaining": 99,
  "retry_after_ms": 0,
  "reset_after_ms": 100
}
```

### Middleware headers

On both success and denial:

- `X-RateLimit-Limit`
- `X-RateLimit-Remaining`
- `Retry-After` (seconds, denial only)

---

## Configuration

Environment variables (see `.env.example`):

| Variable | Default | Description |
|----------|---------|-------------|
| `BIND_ADDR` | `0.0.0.0:8080` | HTTP bind address |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection URL |
| `ALGORITHM` | `token_bucket` | `token_bucket` or `sliding_window` |
| `TB_CAPACITY` | `100` | Token bucket capacity |
| `TB_REFILL_PER_SEC` | `10` | Tokens refilled per second |
| `SW_LIMIT` | `100` | Sliding window max requests |
| `SW_WINDOW_MS` | `60000` | Sliding window size (ms) |
| `FAIL_OPEN` | `true` | Allow traffic if Redis is down |
| `RUST_LOG` | `info` | Tracing filter |

**Key prefix** is a compile-time constant in `src/core/limiter.rs`:

```rust
pub const KEY_PREFIX: &str = "rl";
```

Redis keys look like: `rl:{algorithm}:{client_key}`.

---

## Library usage

```rust
use std::sync::Arc;
use distributed_rate_limiter::{
    Algorithm, RateLimiter, TokenBucket, Memory,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let algo = Algorithm::TokenBucket(TokenBucket::new(100, 10.0)?);
    let storage = Arc::new(Memory::new());
    let limiter = RateLimiter::new(algo, storage)?
        .with_fail_open(true);

    let decision = limiter.check("user:42", 1).await?;
    println!("allowed={}, remaining={}", decision.allowed, decision.remaining);
    Ok(())
}
```

Actix middleware:

```rust
use distributed_rate_limiter::middlewares::rate_limit_middleware::RateLimitMiddleware;

App::new()
    .service(
        web::scope("/api")
            .wrap(RateLimitMiddleware::new(limiter.clone()))
            .route("/hello", web::get().to(hello)),
    )
```

Custom key extractor (e.g. API key header):

```rust
RateLimitMiddleware::new(limiter)
    .with_key_extractor(|req| {
        req.headers()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("anonymous")
            .to_string()
    })
```

---

## Project layout

```
src/
├── algorithms/          # Token Bucket, Sliding Window, Decision
├── core/                # Config, errors, RateLimiter, metrics
├── interceptors/        # Actix ResponseError mapping
├── middlewares/         # RateLimitMiddleware
├── storage/             # IStorage trait, Redis + Memory backends
├── lib.rs
└── main.rs              # HTTP server binary
tests/                   # Integration / unit tests
benches/                 # Microbenchmarks
```

---


## Examples

Runnable library demos (in-memory, no Redis required):

```bash
cargo run --example basic_memory
cargo run --example sliding_window_memory
cargo run --example cost_and_keys
```

| Example | What it shows |
|---------|----------------|
| `basic_memory` | Token Bucket allow / deny cycle |
| `sliding_window_memory` | Sliding Window hard limit |
| `cost_and_keys` | Request cost and per-key isolation |

## Development

```bash
# Format
cargo fmt

# Lint
cargo clippy --all-targets -- -D warnings

# Unit + integration tests (in-memory; no Redis required)
cargo test

# Run with Redis
cargo run

# Benchmarks
cargo bench
```

### Metrics

Scraped at `GET /metrics`. Notable series:

- `ratelimiter_check_duration_seconds` — check latency histogram
- `ratelimiter_decisions_total{algorithm,result}` — allowed / denied
- `ratelimiter_backend_errors_total{algorithm}` — storage failures

---

## Design notes

- **Atomicity**: Redis paths use Lua scripts so check + update is a single round-trip.
- **Cost**: Both algorithms accept a `cost` argument (useful for expensive operations).
- **Fail-open**: Prefer availability under Redis outages; switch to fail-closed for strict quotas.
- **Isolation**: `KEY_PREFIX` + algorithm name prevent key collisions across algorithms and environments.

---

## License

MIT — see [LICENSE](LICENSE).

---

## Author

**Davi Gomes** — portfolio / technical assessment project.
