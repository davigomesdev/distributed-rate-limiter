use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use crate::algorithms::decision::Decision;
use crate::algorithms::sliding_window::SlidingWindow;
use crate::algorithms::token_bucket::TokenBucket;
use crate::core::error::Result;

use super::storage::IStorage;

struct Bucket {
    tokens: f64,
    last_ms: f64,
}

pub struct Memory {
    start: Instant,
    buckets: Mutex<HashMap<String, Bucket>>,
    windows: Mutex<HashMap<String, VecDeque<u64>>>,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            buckets: Mutex::default(),
            windows: Mutex::default(),
        }
    }

    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

#[async_trait]
impl IStorage for Memory {
    async fn token_bucket(&self, key: &str, cfg: &TokenBucket, cost: u64) -> Result<Decision> {
        let now: f64 = self.now_ms() as f64;
        let cap: f64 = cfg.capacity as f64;

        let rate: f64 = cfg.refill_per_ms();
        let cost_f: f64 = cost as f64;

        let mut map: MutexGuard<'_, HashMap<String, Bucket>> = self.buckets.lock().unwrap();
        let b: &mut Bucket = map.entry(key.to_string()).or_insert(Bucket { tokens: cap, last_ms: now });

        b.tokens = (b.tokens + (now - b.last_ms).max(0.0) * rate).min(cap);
        b.last_ms = now;

        let (allowed, retry): (bool, u64) = if b.tokens >= cost_f {
            b.tokens -= cost_f;
            (true, 0)
        } else {
            (false, ((cost_f - b.tokens) / rate).ceil() as u64)
        };

        Ok(Decision {
            allowed,
            limit: cfg.capacity,
            remaining: b.tokens.floor() as u64,
            retry_after_ms: retry,
            reset_after_ms: ((cap - b.tokens) / rate).ceil() as u64,
        })
    }

    async fn sliding_window(&self, key: &str, cfg: &SlidingWindow, cost: u64) -> Result<Decision> {
        let now: u64 = self.now_ms();

        let mut map: MutexGuard<'_, HashMap<String, VecDeque<u64>>> = self.windows.lock().unwrap();
        let log: &mut VecDeque<u64> = map.entry(key.to_string()).or_default();

        while let Some(&front) = log.front() {
            if front + cfg.window_ms <= now {
                log.pop_front();
            } else {
                break;
            }
        }

        let count: u64 = log.len() as u64;
        let retry: u64 = log.front().map(|&t| (t + cfg.window_ms).saturating_sub(now).max(1)).unwrap_or(cfg.window_ms);

        if count + cost > cfg.limit {
            return Ok(Decision {
                allowed: false,
                limit: cfg.limit,
                remaining: cfg.limit.saturating_sub(count),
                retry_after_ms: retry,
                reset_after_ms: retry,
            });
        }

        for _ in 0..cost {
            log.push_back(now);
        }

        Ok(Decision {
            allowed: true,
            limit: cfg.limit,
            remaining: cfg.limit - count - cost,
            retry_after_ms: 0,
            reset_after_ms: retry,
        })
    }
}
