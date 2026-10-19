use std::sync::Arc;

use distributed_rate_limiter::{Algorithm, Decision, Memory, RateLimiter, Result, SlidingWindow, TokenBucket};

fn tb_limiter(capacity: u64, refill_per_sec: f64) -> RateLimiter<Memory> {
    let algo: Algorithm = Algorithm::TokenBucket(TokenBucket::new(capacity, refill_per_sec).unwrap());
    let storage: Arc<Memory> = Arc::new(Memory::new());
    RateLimiter::new(algo, storage).unwrap()
}

fn sw_limiter(limit: u64, window_ms: u64) -> RateLimiter<Memory> {
    let algo: Algorithm = Algorithm::SlidingWindow(SlidingWindow::new(limit, window_ms).unwrap());
    let storage: Arc<Memory> = Arc::new(Memory::new());
    RateLimiter::new(algo, storage).unwrap()
}

#[tokio::test]
async fn token_bucket_allows_within_capacity() {
    let limiter: RateLimiter<Memory> = tb_limiter(5, 1.0);

    for i in 0..5 {
        let d: Decision = limiter.check("k", 1).await.unwrap();
        assert!(d.allowed, "request {i} should be allowed");
        assert_eq!(d.limit, 5u64);
    }

    let denied: Decision = limiter.check("k", 1).await.unwrap();
    assert!(!denied.allowed);
    assert!(denied.retry_after_ms > 0);
}

#[tokio::test]
async fn token_bucket_respects_cost() {
    let limiter: RateLimiter<Memory> = tb_limiter(10, 1.0);

    let d: Decision = limiter.check("k", 7).await.unwrap();
    assert!(d.allowed);
    assert_eq!(d.remaining, 3u64);

    let denied: Decision = limiter.check("k", 5).await.unwrap();
    assert!(!denied.allowed);
}

#[tokio::test]
async fn sliding_window_enforces_limit() {
    let limiter: RateLimiter<Memory> = sw_limiter(3, 60_000);

    for _ in 0..3 {
        let d: Decision = limiter.check("user", 1).await.unwrap();
        assert!(d.allowed);
    }

    let denied: Decision = limiter.check("user", 1).await.unwrap();
    assert!(!denied.allowed);
    assert_eq!(denied.remaining, 0u64);
}

#[tokio::test]
async fn keys_are_isolated() {
    let limiter: RateLimiter<Memory> = tb_limiter(1, 0.001);

    let first_a: Decision = limiter.check("a", 1).await.unwrap();
    assert!(first_a.allowed);

    let first_b: Decision = limiter.check("b", 1).await.unwrap();
    assert!(first_b.allowed);

    let second_a: Decision = limiter.check("a", 1).await.unwrap();
    assert!(!second_a.allowed);
}

#[tokio::test]
async fn invalid_token_bucket_config() {
    let zero_capacity: Result<TokenBucket> = TokenBucket::new(0, 1.0);
    assert!(zero_capacity.is_err());

    let zero_refill: Result<TokenBucket> = TokenBucket::new(10, 0.0);
    assert!(zero_refill.is_err());

    let nan_refill: Result<TokenBucket> = TokenBucket::new(10, f64::NAN);
    assert!(nan_refill.is_err());
}

#[tokio::test]
async fn invalid_sliding_window_config() {
    let zero_limit: Result<SlidingWindow> = SlidingWindow::new(0, 1000);
    assert!(zero_limit.is_err());

    let zero_window: Result<SlidingWindow> = SlidingWindow::new(10, 0);
    assert!(zero_window.is_err());
}

#[tokio::test]
async fn decision_retry_after_secs_rounds_up() {
    let d: Decision = Decision {
        limit: 10,
        allowed: false,
        remaining: 0,
        retry_after_ms: 1500,
        reset_after_ms: 1500,
    };
    let secs: u64 = d.retry_after_secs();
    assert_eq!(secs, 2u64);
}

#[tokio::test]
async fn fail_open_returns_allowed_decision() {
    let d: Decision = Decision::fail_open(100);
    assert!(d.allowed);
    assert_eq!(d.remaining, 100u64);
    assert_eq!(d.retry_after_ms, 0u64);
}
