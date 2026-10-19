use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use tracing::{error, warn};

use crate::core::error::RateLimitError;

#[derive(Serialize)]
struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    pub code: u16,
    pub r#type: String,
    pub messages: Vec<String>,
}

impl ResponseError for RateLimitError {
    fn status_code(&self) -> StatusCode {
        match self {
            RateLimitError::Config(_) => StatusCode::BAD_REQUEST,
            RateLimitError::UnexpectedResponse(_) => StatusCode::BAD_GATEWAY,
            RateLimitError::Redis(_) | RateLimitError::Metrics(_) | RateLimitError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status: StatusCode = self.status_code();

        let (error_type, messages): (&str, Vec<String>) = match self {
            RateLimitError::Config(msg) => {
                warn!(error = %msg, "invalid configuration");
                ("BAD_REQUEST", vec![msg.clone()])
            }

            RateLimitError::UnexpectedResponse(msg) => {
                error!(error = %msg, "unexpected backend response");
                ("BAD_GATEWAY", vec!["unexpected backend response".to_string()])
            }

            RateLimitError::Redis(err) => {
                error!(error = ?err, "redis backend error");
                ("INTERNAL_SERVER_ERROR", vec!["internal server error".to_string()])
            }

            RateLimitError::Metrics(err) => {
                error!(error = ?err, "metrics error");
                ("INTERNAL_SERVER_ERROR", vec!["internal server error".to_string()])
            }

            RateLimitError::InternalServerError => {
                error!("internal server error");
                ("INTERNAL_SERVER_ERROR", vec!["internal server error".to_string()])
            }
        };

        HttpResponse::build(status).json(ErrorBody {
            error: ErrorDetail {
                code: status.as_u16(),
                r#type: error_type.to_string(),
                messages,
            },
        })
    }
}
