#[derive(Debug, thiserror::Error)]
pub enum TurnstileError {
    #[error("Turnstile token not found in request headers")]
    TokenNotFound,

    #[error("Invalid Turnstile token format")]
    InvalidTokenFormat,

    #[error("Client IP address not found")]
    ClientIPNotFound,

    #[error("Turnstile verification failed: {0}")]
    VerificationFailed(String),

    #[error("Network error during Turnstile verification: {0}")]
    NetworkError(#[from] reqwest::Error),
}

impl actix_web::ResponseError for TurnstileError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            TurnstileError::NetworkError(_) => actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
            _ => actix_web::http::StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        let public_message = match self {
            TurnstileError::TokenNotFound | TurnstileError::InvalidTokenFormat => {
                "CAPTCHA verification failed: invalid token"
            }
            TurnstileError::ClientIPNotFound => {
                "CAPTCHA verification failed: client information missing"
            }
            TurnstileError::VerificationFailed(_) => {
                "CAPTCHA verification failed: please try again"
            }
            TurnstileError::NetworkError(_) => "CAPTCHA service temporarily unavailable",
        };

        actix_web::HttpResponse::BadRequest().json(serde_json::json!({
            "error": "captcha_verification_failed",
            "message": public_message
        }))
    }
}
