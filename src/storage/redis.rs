use async_trait::async_trait;
use redis::Script;
use redis::aio::ConnectionManager;

use crate::algorithms::decision::Decision;
use crate::algorithms::sliding_window::{SLIDING_WINDOW_SCRIPT, SlidingWindow};
use crate::algorithms::token_bucket::{TOKEN_BUCKET_SCRIPT, TokenBucket};
use crate::core::error::{RateLimitError, Result};

use super::storage::IStorage;

#[derive(Clone)]
pub struct RedisStorage {
    conn: ConnectionManager,
    token_bucket: Script,
    sliding_window: Script,
}

impl RedisStorage {
    pub async fn connect(url: &str) -> Result<Self> {
        let client: redis::Client = redis::Client::open(url)?;
        let conn: ConnectionManager = ConnectionManager::new(client).await?;

        Ok(Self {
            conn,
            token_bucket: Script::new(TOKEN_BUCKET_SCRIPT),
            sliding_window: Script::new(SLIDING_WINDOW_SCRIPT),
        })
    }

    pub async fn ping(&self) -> Result<()> {
        let mut conn: ConnectionManager = self.conn.clone();
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }
}

#[async_trait]
impl IStorage for RedisStorage {
    async fn token_bucket(&self, key: &str, cfg: &TokenBucket, cost: u64) -> Result<Decision> {
        let mut conn: ConnectionManager = self.conn.clone();
        let reply: Vec<i64> = self.token_bucket.key(key).arg(cfg.capacity).arg(cfg.refill_per_ms()).arg(cost).invoke_async(&mut conn).await?;

        if reply.len() != 4 {
            return Err(RateLimitError::UnexpectedResponse(format!("expected 4 values, received {reply:?}")));
        }

        Ok(Decision {
            allowed: reply[0] == 1,
            limit: cfg.capacity,
            remaining: reply[1].max(0) as u64,
            retry_after_ms: reply[2].max(0) as u64,
            reset_after_ms: reply[3].max(0) as u64,
        })
    }

    async fn sliding_window(&self, key: &str, cfg: &SlidingWindow, cost: u64) -> Result<Decision> {
        let mut conn: ConnectionManager = self.conn.clone();
        let member_prefix: String = uuid::Uuid::new_v4().to_string();

        let reply: Vec<i64> = self.sliding_window.key(key).arg(cfg.limit).arg(cfg.window_ms).arg(cost).arg(member_prefix).invoke_async(&mut conn).await?;

        if reply.len() != 4 {
            return Err(RateLimitError::UnexpectedResponse(format!("expected 4 values, received {reply:?}")));
        }

        Ok(Decision {
            allowed: reply[0] == 1,
            limit: cfg.limit,
            remaining: reply[1].max(0) as u64,
            retry_after_ms: reply[2].max(0) as u64,
            reset_after_ms: reply[3].max(0) as u64,
        })
    }
}
