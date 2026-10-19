use crate::algorithms::decision::Algorithm;
use crate::algorithms::sliding_window::SlidingWindow;
use crate::algorithms::token_bucket::TokenBucket;

use super::error::{RateLimitError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub redis_url: String,
    pub algorithm: Algorithm,
    pub fail_open: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let bind_addr: String = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
        let redis_url: String = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
        let fail_open: bool = std::env::var("FAIL_OPEN").map(|v: String| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(true);

        let algo_name: String = std::env::var("ALGORITHM").unwrap_or_else(|_| "token_bucket".into());

        let algorithm: Algorithm = if algo_name == "sliding_window" {
            let limit: u64 = std::env::var("SW_LIMIT").ok().and_then(|s: String| s.parse::<u64>().ok()).unwrap_or(100);
            let window_ms: u64 = std::env::var("SW_WINDOW_MS").ok().and_then(|s: String| s.parse::<u64>().ok()).unwrap_or(60_000);
            Algorithm::SlidingWindow(SlidingWindow::new(limit, window_ms)?)
        } else if algo_name == "token_bucket" {
            let capacity: u64 = std::env::var("TB_CAPACITY").ok().and_then(|s: String| s.parse::<u64>().ok()).unwrap_or(100);
            let refill_per_sec: f64 = std::env::var("TB_REFILL_PER_SEC").ok().and_then(|s: String| s.parse::<f64>().ok()).unwrap_or(10.0);
            Algorithm::TokenBucket(TokenBucket::new(capacity, refill_per_sec)?)
        } else {
            return Err(RateLimitError::Config(format!("unknown ALGORITHM '{}', use token_bucket or sliding_window", algo_name)));
        };

        Ok(Self { bind_addr, redis_url, algorithm, fail_open })
    }
}
