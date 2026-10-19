use crate::core::error::{RateLimitError, Result};

pub const SLIDING_WINDOW_SCRIPT: &str = r#"
local limit  = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local cost   = tonumber(ARGV[3])
local prefix = ARGV[4]

local t   = redis.call('TIME')
local now = t[1] * 1000 + math.floor(t[2] / 1000)

redis.call('ZREMRANGEBYSCORE', KEYS[1], '-inf', now - window)
local count = redis.call('ZCARD', KEYS[1])

if count + cost <= limit then
  for i = 1, cost do
    redis.call('ZADD', KEYS[1], now, prefix .. ':' .. i)
  end
  redis.call('PEXPIRE', KEYS[1], window)
  local oldest = redis.call('ZRANGE', KEYS[1], 0, 0, 'WITHSCORES')
  local reset = window
  if oldest[2] then reset = math.max(0, tonumber(oldest[2]) + window - now) end
  return {1, limit - count - cost, 0, reset}
end

local oldest = redis.call('ZRANGE', KEYS[1], 0, 0, 'WITHSCORES')
local retry = window
if oldest[2] then retry = math.max(1, tonumber(oldest[2]) + window - now) end
return {0, math.max(0, limit - count), retry, retry}
"#;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlidingWindow {
    pub limit: u64,
    pub window_ms: u64,
}

impl SlidingWindow {
    pub fn new(limit: u64, window_ms: u64) -> Result<Self> {
        if limit == 0 || window_ms == 0 {
            return Err(RateLimitError::Config("limit and window_ms must be > 0".into()));
        }

        Ok(Self { limit, window_ms })
    }
}
