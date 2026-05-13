use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub id: String,
    pub title: String,
    pub url: String,
    pub disk_type: String,
    pub channel: String,
    pub password: String,
    pub created_at: i64,
    pub clicks: u64,
}

#[derive(Clone)]
pub struct ResourceCache {
    resources: Arc<DashMap<String, ResourceInfo>>,
}

impl ResourceCache {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(DashMap::new()),
        }
    }

    pub fn insert(&self, title: &str, url: &str, disk_type: &str, channel: &str, password: &str) -> String {
        let id = short_id(url);

        if let Some(mut entry) = self.resources.get_mut(&id) {
            entry.clicks += 1;
            return id;
        }

        let info = ResourceInfo {
            id: id.clone(),
            title: title.to_string(),
            url: url.to_string(),
            disk_type: disk_type.to_string(),
            channel: channel.to_string(),
            password: password.to_string(),
            created_at: chrono::Utc::now().timestamp(),
            clicks: 1,
        };
        self.resources.insert(id.clone(), info);
        id
    }

    pub fn hot_keywords(&self, n: usize) -> Vec<String> {
        let mut entries: Vec<_> = self.resources.iter().map(|e| e.value().clone()).collect();
        entries.sort_by(|a, b| b.clicks.cmp(&a.clicks));

        let mut seen = std::collections::HashSet::new();
        let mut keywords = Vec::with_capacity(n);
        for info in &entries {
            let title = info.title.trim();
            if title.is_empty() {
                continue;
            }
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
}

fn short_id(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..12].to_string()
}
