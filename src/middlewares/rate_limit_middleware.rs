use actix_web::body::{EitherBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::http::header::{HeaderName, HeaderValue, RETRY_AFTER};
use actix_web::{Error, HttpResponse};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use std::rc::Rc;
use std::sync::Arc;

use crate::core::limiter::RateLimiter;
use crate::storage::storage::IStorage;

pub type KeyExtractor = Arc<dyn Fn(&ServiceRequest) -> String + Send + Sync>;

pub struct RateLimitMiddleware<ST: IStorage + Send + Sync + 'static> {
    key_fn: KeyExtractor,
    limiter: Arc<RateLimiter<ST>>,
}

impl<ST: IStorage + Send + Sync + 'static> RateLimitMiddleware<ST> {
    pub fn new(limiter: Arc<RateLimiter<ST>>) -> Self {
        let default_extractor: KeyExtractor = Arc::new(|req: &ServiceRequest| -> String {
            req.connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown")
                .to_string()
        });

        Self {
            key_fn: default_extractor,
            limiter,
        }
    }

    pub fn with_key_extractor<F>(mut self, f: F) -> Self
    where
        F: Fn(&ServiceRequest) -> String + Send + Sync + 'static,
    {
        self.key_fn = Arc::new(f);
        self
    }
}

pub struct RateLimitMiddlewareService<S, ST: IStorage + Send + Sync + 'static> {
    service: Rc<S>,
    limiter: Arc<RateLimiter<ST>>,
    key_fn: KeyExtractor,
}

impl<S, B, ST> Transform<S, ServiceRequest> for RateLimitMiddleware<ST>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    ST: IStorage + Send + Sync + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RateLimitMiddlewareService<S, ST>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddlewareService {
            service: Rc::new(service),
            key_fn: Arc::clone(&self.key_fn),
            limiter: Arc::clone(&self.limiter),
        }))
    }
}

impl<S, B, ST> Service<ServiceRequest> for RateLimitMiddlewareService<S, ST>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    ST: IStorage + Send + Sync + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service: Rc<S> = Rc::clone(&self.service);
        let key: String = (self.key_fn)(&req);
        let limiter: Arc<RateLimiter<ST>> = Arc::clone(&self.limiter);

        Box::pin(async move {
            match limiter.check(&key, 1).await {
                Ok(decision) if !decision.allowed => {
                    let response: ServiceResponse = req.into_response(
                        HttpResponse::TooManyRequests()
                            .insert_header((RETRY_AFTER, decision.retry_after_secs().to_string()))
                            .insert_header(("x-ratelimit-limit", decision.limit.to_string()))
                            .insert_header(("x-ratelimit-remaining", decision.remaining.to_string()))
                            .json(serde_json::json!({
                                "error": "rate_limit_exceeded",
                                "retry_after_ms": decision.retry_after_ms,
                            })),
                    );
                    Ok(response.map_into_right_body())
                }
                Ok(decision) => {
                    let mut res: ServiceResponse<B> = service.call(req).await?;
                    let headers = res.headers_mut();

                    if let Ok(value) = HeaderValue::from_str(&decision.limit.to_string()) {
                        headers.insert(HeaderName::from_static("x-ratelimit-limit"), value);
                    }
                    if let Ok(value) = HeaderValue::from_str(&decision.remaining.to_string()) {
                        headers.insert(HeaderName::from_static("x-ratelimit-remaining"), value);
                    }

                    Ok(res.map_into_left_body())
                }
                Err(err) => {
                    tracing::error!(error = %err, "rate limiter failure");
                    let response: ServiceResponse =
                        req.into_response(HttpResponse::ServiceUnavailable().finish());
                    Ok(response.map_into_right_body())
                }
            }
        })
    }
}
