mod alupan;
mod jikepan;
mod pan666;
mod yunsou;

use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;

use crate::model::SearchResult;

#[async_trait]
pub trait SearchPlugin: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &str;
    #[allow(dead_code)]
    fn priority(&self) -> i32 {
        3
    }
    async fn search(&self, keyword: &str, client: &Client) -> Vec<SearchResult>;
}

pub struct PluginRegistry {
    plugins: Vec<Arc<dyn SearchPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let mut registry = Self { plugins: Vec::new() };
        registry.register(Arc::new(jikepan::JikepanPlugin));
        registry.register(Arc::new(pan666::Pan666Plugin));
        registry.register(Arc::new(alupan::AlupanPlugin));
        registry.register(Arc::new(yunsou::YunsouPlugin));
        registry
    }

    pub fn register(&mut self, plugin: Arc<dyn SearchPlugin>) {
        self.plugins.push(plugin);
    }

    #[allow(dead_code)]
    pub fn list(&self) -> &[Arc<dyn SearchPlugin>] {
        &self.plugins
    }

    pub async fn search_all(
        &self,
        keyword: &str,
        client: &Client,
    ) -> Vec<SearchResult> {
        let handles: Vec<_> = self
            .plugins
            .iter()
            .map(|p| {
                let p = Arc::clone(p);
                let keyword = keyword.to_string();
                let client = client.clone();
                tokio::spawn(async move { p.search(&keyword, &client).await })
            })
            .collect();

        let mut all_results = Vec::new();
        for handle in handles {
            if let Ok(results) = handle.await {
                all_results.extend(results);
            }
        }
        all_results
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn filter_results_by_keyword(results: &mut Vec<SearchResult>, keyword: &str) {
    if keyword.is_empty() {
        return;
    }
    let lower_keyword = keyword.to_lowercase();
    let keywords: Vec<&str> = lower_keyword.split_whitespace().collect();

    results.retain(|r| {
        let lower_title = r.title.to_lowercase();
        let lower_content = r.content.to_lowercase();
        keywords.iter().all(|kw| lower_title.contains(kw) || lower_content.contains(kw))
    });
}
