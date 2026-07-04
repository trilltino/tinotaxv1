use std::time::Duration;

use anyhow::{bail, Context, Result};
use tracing::warn;

const MAX_ATTEMPTS: u32 = 5;
const BASE_BACKOFF_MS: u64 = 500;

/// Thin reqwest wrapper: JSON GETs with bounded exponential-backoff retry on
/// 429/5xx/transport errors. 4xx other than 429 fails fast — retrying a bad
/// address just hammers the provider.
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("tinotax/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .context("building HTTP client")?;
        Ok(Self { client })
    }

    pub async fn get_json(
        &self,
        url: &str,
        query: &[(String, String)],
        headers: &[(&str, String)],
    ) -> Result<serde_json::Value> {
        self.get_json_with_attempts(url, query, headers, MAX_ATTEMPTS).await
    }

    /// Single-shot variant for health checks (`doctor`) where waiting out
    /// the retry backoff would just be annoying.
    pub async fn get_json_once(
        &self,
        url: &str,
        query: &[(String, String)],
        headers: &[(&str, String)],
    ) -> Result<serde_json::Value> {
        self.get_json_with_attempts(url, query, headers, 1).await
    }

    async fn get_json_with_attempts(
        &self,
        url: &str,
        query: &[(String, String)],
        headers: &[(&str, String)],
        max_attempts: u32,
    ) -> Result<serde_json::Value> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let mut request = self.client.get(url).query(query);
            for (name, value) in headers {
                request = request.header(*name, value);
            }

            let outcome = match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return response
                            .json::<serde_json::Value>()
                            .await
                            .with_context(|| format!("decoding JSON from {url}"));
                    }
                    let retryable = status.as_u16() == 429 || status.is_server_error();
                    if !retryable {
                        let body = response.text().await.unwrap_or_default();
                        bail!("GET {url} failed with {status}: {}", truncate(&body, 300));
                    }
                    format!("{status}")
                }
                Err(err) => format!("transport error: {err}"),
            };

            if attempt >= max_attempts {
                bail!("GET {url} failed after {max_attempts} attempt(s) (last: {outcome})");
            }
            let backoff = Duration::from_millis(BASE_BACKOFF_MS * 2u64.pow(attempt - 1));
            warn!(url, attempt, ?backoff, "retrying after {outcome}");
            tokio::time::sleep(backoff).await;
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
