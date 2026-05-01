use std::{sync::Arc, time::Instant};

use async_trait::async_trait;
use tracing::info;

use crate::model::SearchResponse;

#[async_trait]
pub trait PostSearchPlugin: Send + Sync {
    fn name(&self) -> &str;
    async fn on_search_completed(&self, keyword: &str, response: &SearchResponse);
}

pub struct PostSearchPluginRegistry {
    plugins: Vec<Arc<dyn PostSearchPlugin>>,
}

impl PostSearchPluginRegistry {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn register(&mut self, plugin: Arc<dyn PostSearchPlugin>) {
        self.plugins.push(plugin);
    }

    /// 触发所有后置插件，fire-and-forget 模式，不阻塞搜索响应。
    pub fn fire_all(&self, keyword: &str, response: SearchResponse) {
        let response = Arc::new(response);
        for plugin in &self.plugins {
            let plugin = Arc::clone(plugin);
            let keyword = keyword.to_string();
            let response = Arc::clone(&response);
            let name = plugin.name().to_string();
            tokio::spawn(async move {
                let start = Instant::now();
                plugin.on_search_completed(&keyword, &response).await;
                info!(
                    "后置插件 [{}] 完成，耗时 {:?}",
                    name,
                    start.elapsed()
                );
            });
        }
    }
}

impl Default for PostSearchPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
