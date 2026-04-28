use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".to_string(),
            data: Some(data),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FilterConfig {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SearchRequest {
    #[serde(rename = "kw")]
    pub keyword: String,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(rename = "conc", default)]
    pub concurrency: i32,
    #[serde(rename = "refresh", default)]
    pub force_refresh: bool,
    #[serde(rename = "res", default)]
    pub result_type: String,
    #[serde(rename = "src", default)]
    pub source_type: String,
    #[serde(default)]
    pub plugins: Vec<String>,
    #[serde(default)]
    pub ext: HashMap<String, Value>,
    #[serde(rename = "cloud_types", default)]
    pub cloud_types: Vec<String>,
    #[serde(default)]
    pub filter: Option<FilterConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Link {
    #[serde(rename = "type")]
    pub disk_type: String,
    pub url: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub message_id: String,
    pub unique_id: String,
    pub channel: String,
    pub datetime: DateTime<Utc>,
    pub title: String,
    pub content: String,
    pub links: Vec<Link>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub images: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MergedLink {
    pub url: String,
    pub password: String,
    pub note: String,
    pub datetime: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub images: Vec<String>,
}

pub type MergedLinks = HashMap<String, Vec<MergedLink>>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    pub total: usize,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub results: Vec<SearchResult>,
    #[serde(rename = "merged_by_type", skip_serializing_if = "HashMap::is_empty", default)]
    pub merged_by_type: MergedLinks,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckItem {
    pub disk_type: String,
    pub url: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckRequest {
    pub items: Vec<CheckItem>,
    #[serde(default)]
    pub view_token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckResult {
    pub disk_type: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_url: Option<String>,
    pub state: String,
    pub cache_hit: bool,
    pub checked_at: i64,
    pub expires_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckResponse {
    pub results: Vec<CheckResult>,
}

#[derive(Debug, Deserialize)]
pub struct GoApiResponse<T> {
    pub code: i32,
    #[allow(dead_code)]
    pub message: String,
    pub data: Option<T>,
}


