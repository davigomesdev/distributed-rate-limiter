use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use serde::Deserialize;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use distributed_rate_limiter::middlewares::rate_limit_middleware::RateLimitMiddleware;
use distributed_rate_limiter::storage::redis::RedisStorage;
use distributed_rate_limiter::{Config, RateLimiter};

#[derive(Deserialize)]
struct CheckRequest {
    key: String,
    #[serde(default = "default_cost")]
    cost: u64,
}

fn default_cost() -> u64 {
    1
}

async fn check(limiter: web::Data<RateLimiter<RedisStorage>>, body: web::Json<CheckRequest>) -> impl Responder {
    let cost: u64 = body.cost.max(1);
    match limiter.check(&body.key, cost).await {
        Ok(d) => {
            let status: actix_web::http::StatusCode = if d.allowed { actix_web::http::StatusCode::OK } else { actix_web::http::StatusCode::TOO_MANY_REQUESTS };
            HttpResponse::build(status).json(serde_json::json!({
                "allowed": d.allowed,
                "limit": d.limit,
                "remaining": d.remaining,
                "retry_after_ms": d.retry_after_ms,
                "reset_after_ms": d.reset_after_ms,
            }))
        }
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({ "error": e.to_string() })),
    }
}

async fn hello() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "hello! this route is protected by the rate limiter"
    }))
}

async fn health(storage: web::Data<RedisStorage>) -> impl Responder {
    match storage.ping().await {
        Ok(()) => HttpResponse::Ok().body("ok"),
        Err(_) => HttpResponse::ServiceUnavailable().body("redis unavailable"),
    }
}

async fn metrics(limiter: web::Data<RateLimiter<RedisStorage>>) -> impl Responder {
    match limiter.metrics().render() {
        Ok(text) => HttpResponse::Ok().content_type("text/plain; version=0.0.4").body(text),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))).init();
    let cfg: Config = Config::from_env().expect("invalid configuration");

    tracing::info!(
        algorithm = cfg.algorithm.name(),
        bind = %cfg.bind_addr,
        "starting"
    );

    let storage: RedisStorage = RedisStorage::connect(&cfg.redis_url).await.expect("failed to connect to Redis");
    let limiter: Arc<RateLimiter<RedisStorage>> = Arc::new(RateLimiter::new(cfg.algorithm, Arc::new(storage.clone())).expect("failed to create rate limiter").with_fail_open(cfg.fail_open));

    let limiter_data: web::Data<RateLimiter<RedisStorage>> = web::Data::from(limiter.clone());
    let storage_data: web::Data<RedisStorage> = web::Data::new(storage);

    HttpServer::new(move || {
        App::new()
            .app_data(limiter_data.clone())
            .app_data(storage_data.clone())
            .route("/health", web::get().to(health))
            .route("/metrics", web::get().to(metrics))
            .route("/v1/check", web::post().to(check))
            .service(web::scope("/api").wrap(RateLimitMiddleware::new(limiter.clone())).route("/hello", web::get().to(hello)))
    })
    .bind(&cfg.bind_addr)?
    .run()
    .await
}
