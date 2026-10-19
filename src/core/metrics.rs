use super::error::Result;
use prometheus::{Encoder, Histogram, HistogramOpts, IntCounterVec, Opts, Registry, TextEncoder};

pub struct Metrics {
    latency: Histogram,
    registry: Registry,
    errors: IntCounterVec,
    decisions: IntCounterVec,
}

impl Metrics {
    pub fn new() -> Result<Self> {
        let latency: Histogram = Histogram::with_opts(HistogramOpts::new("check_duration_seconds", "Latency of a rate limit check").buckets(vec![0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1]))?;
        let registry: Registry = Registry::new_custom(Some("ratelimiter".into()), None)?;

        let errors: IntCounterVec = IntCounterVec::new(Opts::new("backend_errors_total", "Errors when querying the backend"), &["algorithm"])?;
        let decisions: IntCounterVec = IntCounterVec::new(Opts::new("decisions_total", "Total rate limiter decisions"), &["algorithm", "result"])?;

        registry.register(Box::new(latency.clone()))?;
        registry.register(Box::new(errors.clone()))?;
        registry.register(Box::new(decisions.clone()))?;

        Ok(Self { registry, decisions, errors, latency })
    }

    pub fn render(&self) -> Result<String> {
        let mut buf: Vec<u8> = Vec::new();
        TextEncoder::new().encode(&self.registry.gather(), &mut buf)?;
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    pub fn record_error(&self, algorithm: &str) {
        self.errors.with_label_values(&[algorithm]).inc();
    }

    pub fn record_decision(&self, algorithm: &str, allowed: bool) {
        let result: &str = if allowed { "allowed" } else { "denied" };
        self.decisions.with_label_values(&[algorithm, result]).inc();
    }

    pub fn observe_latency(&self, seconds: f64) {
        self.latency.observe(seconds);
    }
}
