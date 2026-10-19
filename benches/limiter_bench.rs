use std::sync::Arc;
use std::time::{Duration, Instant};

use distributed_rate_limiter::{Algorithm, Memory, RateLimiter, TokenBucket};

fn main() {
    let algo: Algorithm = Algorithm::TokenBucket(TokenBucket::new(1_000_000, 1_000_000.0).unwrap());
    let limiter: RateLimiter<Memory> = RateLimiter::new(algo, Arc::new(Memory::new())).unwrap();

    let iterations: u64 = 50_000;
    let start: Instant = Instant::now();

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

    rt.block_on(async {
        for i in 0..iterations {
            let _ = limiter.check(&format!("key-{i}"), 1).await.unwrap();
        }
    });

    let elapsed: Duration = start.elapsed();
    let per_sec: f64 = iterations as f64 / elapsed.as_secs_f64();

    println!("memory token_bucket: {iterations} checks in {elapsed:?} ({per_sec:.0} checks/s)");
}
