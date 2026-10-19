use super::sliding_window::SlidingWindow;
use super::token_bucket::TokenBucket;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Algorithm {
    TokenBucket(TokenBucket),
    SlidingWindow(SlidingWindow),
}

impl Algorithm {
    pub fn name(&self) -> &'static str {
        match self {
            Algorithm::TokenBucket(_) => "token_bucket",
            Algorithm::SlidingWindow(_) => "sliding_window",
        }
    }

    pub fn limit(&self) -> u64 {
        match self {
            Algorithm::TokenBucket(c) => c.capacity,
            Algorithm::SlidingWindow(c) => c.limit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub limit: u64,
    pub allowed: bool,
    pub remaining: u64,
    pub retry_after_ms: u64,
    pub reset_after_ms: u64,
}

impl Decision {
    pub fn fail_open(limit: u64) -> Self {
        Self {
            limit,
            allowed: true,
            remaining: limit,
            retry_after_ms: 0,
            reset_after_ms: 0,
        }
    }

    pub fn retry_after_secs(&self) -> u64 {
        self.retry_after_ms.div_ceil(1000)
    }
}
