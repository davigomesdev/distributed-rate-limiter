use crate::core::error::{RateLimitError, Result};

pub const TOKEN_BUCKET_SCRIPT: &str = r#"
local capacity = tonumber(ARGV[1])
local refill   = tonumber(ARGV[2])
local cost     = tonumber(ARGV[3])

local t   = redis.call('TIME')
local now = t[1] * 1000 + math.floor(t[2] / 1000)

local data   = redis.call('HMGET', KEYS[1], 'tokens', 'ts')
local tokens = tonumber(data[1])
local ts     = tonumber(data[2])
if tokens == nil or ts == nil then
  tokens = capacity
  ts = now
end

local delta = math.max(0, now - ts)
tokens = math.min(capacity, tokens + delta * refill)

local allowed = 0
local retry = 0
if tokens >= cost then
  tokens = tokens - cost
  allowed = 1
else
  retry = math.ceil((cost - tokens) / refill)
end

redis.call('HSET', KEYS[1], 'tokens', tostring(tokens), 'ts', now)
redis.call('PEXPIRE', KEYS[1], math.ceil(capacity / refill) + 1000)

local reset = math.ceil((capacity - tokens) / refill)
return {allowed, math.floor(tokens), retry, reset}
"#;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenBucket {
    pub capacity: u64,
    pub refill_per_sec: f64,
}

impl TokenBucket {
    pub fn new(capacity: u64, refill_per_sec: f64) -> Result<Self> {
        if capacity == 0 {
            return Err(RateLimitError::Config("capacity must be > 0".into()));
        }
        if !refill_per_sec.is_finite() || refill_per_sec <= 0.0 {
            return Err(RateLimitError::Config("refill_per_sec must be finite and > 0".into()));
        }

        Ok(Self { capacity, refill_per_sec })
    }

    pub fn refill_per_ms(&self) -> f64 {
        self.refill_per_sec / 1000.0
    }
}
