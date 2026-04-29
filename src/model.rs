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
pub struct SearchResponse {
    pub total: usize,
    pub cache_hit: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub results: Vec<SearchResult>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let resp = ApiResponse::success("hello");
        assert_eq!(resp.code, 0);
        assert_eq!(resp.message, "success");
        assert_eq!(resp.data, Some("hello"));
    }

    #[test]
    fn test_search_request_default() {
        let req = SearchRequest::default();
        assert!(req.keyword.is_empty());
        assert!(req.channels.is_empty());
        assert!(!req.force_refresh);
        assert_eq!(req.source_type, "");
        assert_eq!(req.result_type, "");
    }

    #[test]
    fn test_search_request_deserialize_minimal() {
        let json = r#"{"kw":"test"}"#;
        let req: SearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.keyword, "test");
        assert!(req.channels.is_empty());
        assert!(!req.force_refresh);
    }

    #[test]
    fn test_search_request_deserialize_full() {
        let json = r#"{"kw":"test","channels":["ch1","ch2"],"conc":5,"refresh":true,"res":"all","src":"tg","cloud_types":["baidu"],"plugins":["p1"]}"#;
        let req: SearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.keyword, "test");
        assert_eq!(req.channels, vec!["ch1", "ch2"]);
        assert_eq!(req.concurrency, 5);
        assert!(req.force_refresh);
        assert_eq!(req.result_type, "all");
        assert_eq!(req.source_type, "tg");
        assert_eq!(req.cloud_types, vec!["baidu"]);
        assert_eq!(req.plugins, vec!["p1"]);
    }

    #[test]
    fn test_link_serialization() {
        let link = Link {
            disk_type: "baidu".into(),
            url: "https://pan.baidu.com/s/abc".into(),
            password: "pwd123".into(),
            datetime: None,
            work_title: None,
        };
        let json = serde_json::to_string(&link).unwrap();
        assert!(json.contains("\"disk_type\":\"baidu\""));
        assert!(json.contains("\"url\":\"https://pan.baidu.com/s/abc\""));
        // None fields should be absent
        assert!(!json.contains("datetime"));
        assert!(!json.contains("work_title"));
    }

    #[test]
    fn test_search_response_serialization() {
        let resp = SearchResponse {
            total: 0,
            cache_hit: false,
            results: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"total\":0"));
        // Empty fields with skip_serializing_if should be absent
        assert!(!json.contains("results"));
    }

    #[test]
    fn test_check_item_deserialize() {
        let json = r#"{"disk_type":"baidu","url":"https://pan.baidu.com/s/abc","password":"1234"}"#;
        let item: CheckItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.disk_type, "baidu");
        assert_eq!(item.url, "https://pan.baidu.com/s/abc");
        assert_eq!(item.password, "1234");
    }
}
