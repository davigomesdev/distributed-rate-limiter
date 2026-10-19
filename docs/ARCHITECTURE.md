# Architecture

## Goals

- Correct, atomic rate-limit decisions under concurrent load.
- Shared limits across multiple service instances (Redis).
- Clear separation between algorithms, storage, and HTTP integration.
- Observable behavior (Prometheus) and controlled failure modes (fail-open / fail-closed).

## Components

### Algorithms (`src/algorithms`)

- `TokenBucket` — continuous refill; decisions return remaining tokens and retry delay.
- `SlidingWindow` — count of events in `[now - window, now]`; Redis uses a sorted set.
- `Decision` — uniform result type for all algorithms.
- `Algorithm` — enum selecting the active strategy at runtime.

### Core (`src/core`)

- `RateLimiter` — orchestrates algorithm + storage, applies `KEY_PREFIX`, records metrics, applies fail-open policy.
- `Config` — environment-driven process configuration (bind address, Redis URL, algorithm params).
- `Metrics` — Prometheus registry (latency, decisions, backend errors).
- `RateLimitError` — typed errors mapped to HTTP by the interceptor.

### Storage (`src/storage`)

- `IStorage` — async trait for algorithm execution against a backend.
- `RedisStorage` — connection manager + Lua scripts (single round-trip check/update).
- `Memory` — mutex-guarded maps for tests and local demos.

### HTTP layer

- `main` binary — Actix server wiring routes and middleware.
- `RateLimitMiddleware` — extract key → check → allow or 429 with standard headers.
- `error_filter` — maps `RateLimitError` to JSON error bodies.

## Key design choices

1. **Lua scripts on Redis** avoid race conditions between read and write of rate-limit state.
2. **Cost parameter** allows charging more than one unit per request (e.g. expensive endpoints).
3. **Compile-time `KEY_PREFIX`** keeps namespaces consistent and avoids accidental key collisions.
4. **Fail-open default** prioritizes availability; operators can set `FAIL_OPEN=false` for strict quotas.
5. **Trait-based storage** keeps unit tests free of Redis while production uses the shared backend.

## Data flow (middleware)

```
Client request
    → key extractor (IP or custom)
    → RateLimiter::check(key, cost=1)
        → full_key = KEY_PREFIX : algorithm : key
        → storage.token_bucket | storage.sliding_window
        → metrics (latency + decision)
    → 429 + Retry-After | forward + X-RateLimit-* headers
```
