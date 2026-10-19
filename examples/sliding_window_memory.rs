use distributed_rate_limiter::{Algorithm, Decision, Memory, RateLimiter, Result, SlidingWindow};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let limit: u64 = 3;
    let window_ms: u64 = 60_000;

    let algorithm: Algorithm = Algorithm::SlidingWindow(SlidingWindow::new(limit, window_ms)?);
    let storage: Arc<Memory> = Arc::new(Memory::new());

    let limiter: RateLimiter<Memory> = RateLimiter::new(algorithm, storage)?.with_fail_open(true);

    println!("Sliding Window — limit={limit}, window_ms={window_ms}\n");

    for i in 1..=5 {
        let decision: Decision = limiter.check("api-key:abc", 1).await?;
        println!("request {i}: allowed={} remaining={} retry_after_ms={}", decision.allowed, decision.remaining, decision.retry_after_ms);
    }

    Ok(())
}
