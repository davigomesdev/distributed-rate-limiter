use thiserror::Error;

#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("Redis backend error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("metrics error: {0}")]
    Metrics(#[from] prometheus::Error),

    #[error("unexpected backend response: {0}")]
    UnexpectedResponse(String),

    #[error("internal server error")]
    InternalServerError,
}

pub type Result<T> = std::result::Result<T, RateLimitError>;
