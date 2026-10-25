use std::sync::Arc;

use anyhow::Result;
use reqwest::{
    Client,
    Response,
};
use serde::Serialize;
use tokio::{
    sync::Mutex,
    time::{
        Duration,
        Instant,
        sleep_until,
    },
};

/// 3 requests per second → one request every 334 ms.
const RATE_INTERVAL: Duration = Duration::from_millis(334);
const BASE_URL: &str = "https://api.warframe.market";

pub struct RateLimitedClient {
    inner: Client,
    /// Earliest moment the next request may be sent.
    next_at: Arc<Mutex<Instant>>,
}

impl RateLimitedClient {
    pub fn new() -> Self {
        Self {
            inner: Client::builder()
                .user_agent("wfmq-cli/0.1.0")
                .build()
                .expect("failed to build HTTP client"),
            next_at: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Acquire the rate-limit slot.
    /// All callers share the same `next_at` mutex, so concurrent calls are
    /// serialised automatically and the limit is never exceeded.
    async fn wait_for_slot(&self) {
        let mut next = self.next_at.lock().await;
        let now = Instant::now();
        if *next > now {
            sleep_until(*next).await;
        }
        // Reserve the next slot before releasing the lock so that a second
        // concurrent caller cannot grab the same window.
        *next = Instant::now() + RATE_INTERVAL;
    }

    /// Acquire the rate-limit slot, then fire the request.
    pub async fn get(
        &self,
        path: &str,
    ) -> Result<Response> {
        self.wait_for_slot().await;

        let resp = self
            .inner
            .get(format!("{BASE_URL}{path}"))
            .header("Language", "en")
            .header("Platform", "pc")
            .header("Accept", "application/json")
            .send()
            .await?;

        Ok(resp)
    }

    /// Authenticated POST with a JSON body. The caller inspects the status.
    pub async fn post_json<B: Serialize + ?Sized>(
        &self,
        path: &str,
        token: &str,
        body: &B,
    ) -> Result<Response> {
        self.wait_for_slot().await;

        let resp = self
            .inner
            .post(format!("{BASE_URL}{path}"))
            .header("Language", "en")
            .header("Platform", "pc")
            .header("Accept", "application/json")
            .bearer_auth(token)
            .json(body)
            .send()
            .await?;

        Ok(resp)
    }
}
