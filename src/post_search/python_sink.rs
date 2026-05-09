use std::time::Instant;

use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use tracing::{error, info};

use crate::model::{SearchResponse, SearchResult};

use super::PostSearchPlugin;

#[derive(Serialize)]
struct IngestPayload<'a> {
    keyword: &'a str,
    results: &'a [SearchResult],
}

pub struct PythonSinkPlugin {
    endpoint: String,
    client: Client,
}

impl PythonSinkPlugin {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            client: Client::builder()
                .user_agent("pansou-python-sink/1.0")
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl PostSearchPlugin for PythonSinkPlugin {
    fn name(&self) -> &str {
        "python_sink"
    }

    async fn on_search_completed(&self, keyword: &str, response: &SearchResponse) {
        if response.results.is_empty() {
            return;
        }

        let start = Instant::now();
        let payload = IngestPayload {
            keyword,
            results: &response.results,
        };

        match self
            .client
            .post(&self.endpoint)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    info!(
                        "python_sink 推送成功 {}ms: keyword={}, total={}, status={}",
                        start.elapsed().as_millis(), keyword, response.total, status
                    );
                } else {
                    error!(
                        "python_sink 推送失败 {}ms: keyword={}, status={}, body={:?}",
                        start.elapsed().as_millis(),
                        keyword,
                        status,
                        resp.text().await.unwrap_or_default()
                    );
                }
            }
            Err(e) => {
                error!("python_sink 请求失败 {}ms: keyword={}, err={}", start.elapsed().as_millis(), keyword, e);
            }
        }
    }
}
