use distributed_rate_limiter::{Algorithm, Decision, Memory, RateLimiter, Result, TokenBucket};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let capacity: u64 = 5;
    let refill_per_sec: f64 = 1.0;

    let algorithm: Algorithm = Algorithm::TokenBucket(TokenBucket::new(capacity, refill_per_sec)?);
    let storage: Arc<Memory> = Arc::new(Memory::new());

    let limiter: RateLimiter<Memory> = RateLimiter::new(algorithm, storage)?.with_fail_open(true);

    println!("Token Bucket — capacity={capacity}, refill={refill_per_sec}/s\n");

    for i in 1..=7 {
        let decision: Decision = limiter.check("user:42", 1).await?;
        println!("request {i}: allowed={} remaining={} retry_after_ms={}", decision.allowed, decision.remaining, decision.retry_after_ms);
    }

    Ok(())
}
