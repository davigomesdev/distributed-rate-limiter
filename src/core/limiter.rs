use std::sync::Arc;
use std::time::Instant;

use crate::algorithms::decision::{Algorithm, Decision};
use crate::storage::storage::IStorage;

use super::error::Result;
use super::metrics::Metrics;

pub const KEY_PREFIX: &str = "rl";

pub struct RateLimiter<S: IStorage + Send + Sync + 'static> {
    storage: Arc<S>,
    algorithm: Algorithm,
    key_prefix: String,
    fail_open: bool,
    metrics: Arc<Metrics>,
}

impl<S: IStorage + Send + Sync + 'static> RateLimiter<S> {
    pub fn new(algorithm: Algorithm, storage: Arc<S>) -> Result<Self> {
        Ok(Self {
            storage,
            algorithm,
            key_prefix: KEY_PREFIX.to_string(),
            fail_open: true,
            metrics: Arc::new(Metrics::new()?),
        })
    }

    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = prefix.into();
        self
    }

    pub fn with_fail_open(mut self, fail_open: bool) -> Self {
        self.fail_open = fail_open;
        self
    }

    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    pub fn metrics(&self) -> &Arc<Metrics> {
        &self.metrics
    }

    pub async fn check(&self, key: &str, cost: u64) -> Result<Decision> {
        let full_key: String = format!("{}:{}:{}", self.key_prefix, self.algorithm.name(), key);
        let started: Instant = Instant::now();

        let result: Result<Decision> = match &self.algorithm {
            Algorithm::TokenBucket(cfg) => self.storage.token_bucket(&full_key, cfg, cost).await,
            Algorithm::SlidingWindow(cfg) => self.storage.sliding_window(&full_key, cfg, cost).await,
        };

        self.metrics.observe_latency(started.elapsed().as_secs_f64());

        match result {
            Ok(decision) => {
                self.metrics.record_decision(self.algorithm.name(), decision.allowed);
                Ok(decision)
            }
            Err(err) => {
                self.metrics.record_error(self.algorithm.name());
                if self.fail_open {
                    tracing::warn!(error = %err, "backend unavailable; allowing (fail-open)");
                    self.metrics.record_decision(self.algorithm.name(), true);
                    Ok(Decision::fail_open(self.algorithm.limit()))
                } else {
                    Err(err)
                }
            }
        }
    }
}
