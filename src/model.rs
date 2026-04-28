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
    pub result_type: String, // all: 全部 (结果 + 按类型合并), results: 结果, 其它: 按类型合并
    #[serde(rename = "src", default)] // all: 全部 (tg + 插件), tg: tg, plugin: 插件
    pub source_type: String, // all: 全部 (tg + 插件), tg: tg, plugin: 插件
    #[serde(default)]
    pub plugins: Vec<String>,
    #[serde(default)]
    pub ext: HashMap<String, Value>,
    #[serde(rename = "cloud_types", default)]
    pub cloud_types: Vec<String>,
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
    pub unique_id: String, // 唯一标识，用于合并搜索结果
    pub channel: String, // tg:xxx, plugin:插件的名字(如panshushu), unknown(默认)
    pub datetime: DateTime<Utc>, // 发布时间
    pub title: String, // 标题
    pub content: String, // 内容
    pub links: Vec<Link>, // 链接
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>, // 标签
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub images: Vec<String>, // 图片
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


