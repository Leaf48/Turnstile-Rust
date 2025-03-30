use serde_json::{json, Value};

use crate::{reqwest_client::REQWEST_CLIENT, TurnstileConfig};

pub async fn verify_cloudflare_turnstile(
    token: &str,
    remoteip: &str,
    config: &TurnstileConfig,
) -> Result<bool, reqwest::Error> {
    let body = json!({
        "secret": config.secret_key,
        "response": token,
        "remoteip": remoteip
    });

    let resp = REQWEST_CLIENT
        .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
        .json(&body)
        .send()
        .await?;

    let js: Value = resp.json().await?;

    Ok(js["success"].as_bool().unwrap_or(false))
}
