use distributed_rate_limiter::{Algorithm, Decision, Memory, RateLimiter, Result, TokenBucket};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let capacity: u64 = 10;
    let refill_per_sec: f64 = 0.001;

    let algorithm: Algorithm = Algorithm::TokenBucket(TokenBucket::new(capacity, refill_per_sec)?);
    let storage: Arc<Memory> = Arc::new(Memory::new());
    let limiter: RateLimiter<Memory> = RateLimiter::new(algorithm, storage)?;

    println!("=== Cost (expensive operation costs 7 tokens) ===\n");

    let cheap: Decision = limiter.check("endpoint:/search", 1).await?;
    println!("cost=1  allowed={} remaining={}", cheap.allowed, cheap.remaining);

    let expensive: Decision = limiter.check("endpoint:/search", 7).await?;
    println!("cost=7  allowed={} remaining={}", expensive.allowed, expensive.remaining);

    let too_expensive: Decision = limiter.check("endpoint:/search", 5).await?;
    println!("cost=5  allowed={} remaining={} retry_after_ms={}", too_expensive.allowed, too_expensive.remaining, too_expensive.retry_after_ms);

    println!("\n=== Key isolation (separate buckets per key) ===\n");

    let a: Decision = limiter.check("user:alice", 1).await?;
    let b: Decision = limiter.check("user:bob", 1).await?;
    println!("alice allowed={} remaining={}", a.allowed, a.remaining);
    println!("bob   allowed={} remaining={}", b.allowed, b.remaining);

    Ok(())
}
