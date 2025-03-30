use std::time::Duration;

use once_cell::sync::Lazy;

pub static REQWEST_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build reqwest client")
});
