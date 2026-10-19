use async_trait::async_trait;

use crate::algorithms::decision::Decision;
use crate::algorithms::sliding_window::SlidingWindow;
use crate::algorithms::token_bucket::TokenBucket;
use crate::core::error::Result;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait IStorage: Send + Sync + 'static {
    async fn token_bucket(&self, key: &str, cfg: &TokenBucket, cost: u64) -> Result<Decision>;
    async fn sliding_window(&self, key: &str, cfg: &SlidingWindow, cost: u64) -> Result<Decision>;
}
