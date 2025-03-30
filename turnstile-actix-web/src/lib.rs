use std::future::{ready, Ready};

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};

use error::TurnstileError;
use futures_util::future::LocalBoxFuture;
use turnstile::verify_cloudflare_turnstile;

pub mod error;
pub mod reqwest_client;
pub mod turnstile;

#[derive(Clone)]
pub struct TurnstileConfig {
    pub secret_key: String,
    pub timeout_secs: Option<u64>,
}

impl TurnstileConfig {
    pub fn new(secret_key: impl Into<String>) -> Self {
        Self {
            secret_key: secret_key.into(),
            timeout_secs: Some(5),
        }
    }
}

pub struct Turnstile {
    config: TurnstileConfig,
}
impl Turnstile {
    pub fn new(config: TurnstileConfig) -> Self {
        Self { config }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Turnstile
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TurnstileMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let config = self.config.clone();
        ready(Ok(TurnstileMiddleware { service, config }))
    }
}

pub struct TurnstileMiddleware<S> {
    service: S,
    config: TurnstileConfig,
}

impl<S, B> Service<ServiceRequest> for TurnstileMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let connection_info = req.connection_info().to_owned();
        let client_ip = match connection_info.realip_remote_addr() {
            Some(ip) => ip.to_owned(),
            None => {
                return Box::pin(async { Err(Error::from(TurnstileError::ClientIPNotFound)) });
            }
        };

        let headers = req.headers();
        let cf_turnstile_response = match headers.get("cf-turnstile-response") {
            Some(res) => match res.to_str() {
                Ok(res) => res.to_owned(),
                Err(_) => {
                    return Box::pin(async {
                        Err(Error::from(TurnstileError::InvalidTokenFormat))
                    });
                }
            },
            None => {
                return Box::pin(async { Err(Error::from(TurnstileError::TokenNotFound)) });
            }
        };
        // println!("{}: {}", client_ip, cf_turnstile_response);

        let fut = self.service.call(req);

        let config = self.config.clone();

        Box::pin(async move {
            match verify_cloudflare_turnstile(&cf_turnstile_response, &client_ip, &config).await {
                Ok(true) => {
                    // success
                    let res = fut.await?;
                    Ok(res)
                }
                Ok(false) => {
                    // cloudflare returned failure
                    Err(Error::from(TurnstileError::VerificationFailed(
                        "Cloudflare rejected the token".to_string(),
                    )))
                }
                Err(err) => {
                    // network error
                    Err(Error::from(TurnstileError::NetworkError(err)))
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{http::header, test, web, App, HttpResponse};

    use super::*;

    #[actix_web::test]
    async fn test_turnstile_success() {
        // Setting for test
        let turnstile_config = TurnstileConfig::new("1x0000000000000000000000000000000AA");

        // Create mock server
        let app =
            test::init_service(App::new().wrap(Turnstile::new(turnstile_config)).service(
                web::resource("/").to(|| async { HttpResponse::Ok().body("hello world") }),
            ))
            .await;

        // Mock sample verification token
        // this is supposed to be returned by a client
        let token = "valid_turnstile_token";

        // Build test request
        let req = test::TestRequest::get()
            .uri("/")
            .insert_header((
                header::HeaderName::from_static("cf-turnstile-response"),
                token,
            ))
            .peer_addr("192.168.1.1:12345".parse().unwrap())
            .to_request();

        // check request
        let resp = test::call_service(&app, req).await;

        // check if success
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_turnstile_failure() {
        // Setting for test
        let turnstile_config = TurnstileConfig::new("2x0000000000000000000000000000000AA");

        // Create mock server
        let app =
            test::init_service(App::new().wrap(Turnstile::new(turnstile_config)).service(
                web::resource("/").to(|| async { HttpResponse::Ok().body("hello world") }),
            ))
            .await;

        // Mock sample verification token
        // this is supposed to be returned by a client
        let token = "valid_turnstile_token";

        // Build test request
        let req = test::TestRequest::get()
            .uri("/")
            .insert_header((
                header::HeaderName::from_static("cf-turnstile-response"),
                token,
            ))
            .peer_addr("192.168.1.1:12345".parse().unwrap())
            .to_request();

        // リクエストの実行と検証
        let resp = test::try_call_service(&app, req).await;
        match resp {
            Ok(response) => {
                println!("{:?}", response);
                assert!(response.status().is_client_error());
            }
            Err(e) => {
                if let Some(turnstile_error) = e.as_error::<TurnstileError>() {
                    match turnstile_error {
                        TurnstileError::VerificationFailed(_) => {
                            println!("{}", e.to_string());
                        }
                        err => {
                            panic!("Unexpected error type: {}", err)
                        }
                    }
                } else {
                    panic!("Unexpected error type: {:?}", e)
                }
            }
        }
    }
}
