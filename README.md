# Turnstile-Rust

Middleware for Actix Web

# Usage

Import and initialize the middleware in your Actix Web application as shown below:

```rust
use actix_web::{web, App, HttpResponse, HttpServer};
use actix_turnstile::{Turnstile, TurnstileConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the Turnstile configuration with your Cloudflare secret key.
    let turnstile_config = TurnstileConfig::new("your_secret_key_here");

    // Build the Actix Web server.
    HttpServer::new(move || {
        App::new()
            // Register the Turnstile middleware.
            .wrap(Turnstile::new(turnstile_config.clone()))
            // Define your routes.
            .service(
                web::resource("/")
                    .to(|| async { HttpResponse::Ok().body("This route is protected by Cloudflare Turnstile") }),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```
