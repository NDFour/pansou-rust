use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub id: String,
    pub title: String,
    pub url: String,
    pub disk_type: String,
    pub channel: String,
    pub password: String,
    pub created_at: i64,
}

#[derive(Clone)]
pub struct ResourceCache {
    resources: Arc<DashMap<String, ResourceInfo>>,
    persist_path: Arc<Mutex<Option<PathBuf>>>,
}

impl ResourceCache {
    pub fn new(persist_path: Option<PathBuf>) -> Self {
        let cache = Self {
            resources: Arc::new(DashMap::new()),
            persist_path: Arc::new(Mutex::new(persist_path)),
        };

        // 从磁盘恢复数据
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            let cache_clone = cache.clone();
            handle.spawn(async move {
                cache_clone.load_from_disk().await;
            });
        }

        cache
    }

    pub fn insert(&self, title: &str, url: &str, disk_type: &str, channel: &str, password: &str) -> String {
        let id = short_id(url);
        let info = ResourceInfo {
            id: id.clone(),
            title: title.to_string(),
            url: url.to_string(),
            disk_type: disk_type.to_string(),
            channel: channel.to_string(),
            password: password.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };
        self.resources.insert(id.clone(), info);

        // 触发异步持久化
        self.schedule_persist();

        id
    }

    pub fn hot_keywords(&self, n: usize) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut keywords = Vec::with_capacity(n);
        for entry in self.resources.iter() {
            let title = entry.value().title.trim();
            if title.is_empty() {
                continue;
            }
            // 取前 15 个字符作为热词，去重
            let end = title.char_indices()
                .take(15)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(title.len());
            let kw = &title[..end.min(title.len())];
            if seen.insert(kw.to_string()) {
                keywords.push(kw.to_string());
            }
            if keywords.len() >= n {
                break;
            }
        }
        keywords
    }

    fn schedule_persist(&self) {
        let cache = self.resources.clone();
        let path = self.persist_path.clone();
        tokio::spawn(async move {
            let path_guard = path.lock().await;
            if let Some(ref file_path) = *path_guard {
                let resources: Vec<ResourceInfo> = cache.iter().map(|r| r.value().clone()).collect();
                if let Ok(json) = serde_json::to_string_pretty(&resources) {
                    let _ = std::fs::write(file_path, json);
                }
            }
        });
    }

    async fn load_from_disk(&self) {
        let path_guard = self.persist_path.lock().await;
        if let Some(ref file_path) = *path_guard {
            if let Ok(data) = std::fs::read_to_string(file_path) {
                if let Ok(resources) = serde_json::from_str::<Vec<ResourceInfo>>(&data) {
                    for r in resources {
                        self.resources.insert(r.id.clone(), r);
                    }
                }
            }
        }
    }
}

fn short_id(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..12].to_string()
}
