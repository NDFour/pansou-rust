use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

use crate::model::{Link, SearchResult};

use super::{filter_results_by_keyword, SearchPlugin};

pub struct YunsouPlugin;

const SEARCH_URL_TEMPLATE: &str = "https://wpys.cc/s/%s.html";

#[derive(Deserialize)]
struct YunsouItem {
    id: i64,
    title: String,
    #[serde(rename = "is_type")]
    is_type: i32,
    #[serde(default)]
    code: Option<String>,
    url: String,
    #[serde(default)]
    times: String,
    #[serde(default)]
    category: YunsouCategory,
}

#[derive(Deserialize, Default)]
struct YunsouCategory {
    #[serde(default)]
    name: String,
}

#[async_trait]
impl SearchPlugin for YunsouPlugin {
    fn name(&self) -> &str {
        "yunsou"
    }

    fn priority(&self) -> i32 {
        2
    }

    async fn search(&self, keyword: &str, client: &Client) -> Vec<SearchResult> {
        debug!(">>> {} 开始搜索, keyword: {}", self.name(), keyword);
        let search_url = SEARCH_URL_TEMPLATE.replace("%s", &urlencoding(keyword));
        let resp = match client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Referer", "https://yunsou.xyz/")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        debug!("<<< {} 响应 status code: {}", self.name(), resp.status());

        let body = match resp.text().await {
            Ok(b) => b,
            Err(_) => return vec![],
        };

        let json_data_re = Regex::new(r"var jsonData = '(.+?)';").unwrap();
        let control_re = Regex::new(r"[\x00-\x1F\x7F]").unwrap();

        let json_str = match json_data_re.captures(&body).and_then(|c| c.get(1)) {
            Some(m) => {
                let raw = m.as_str();
                let cleaned = control_re.replace_all(raw, "");
                cleaned.replace("\\/", "/")
            }
            None => return vec![],
        };

        let items: Vec<YunsouItem> = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        let mut results: Vec<SearchResult> = items
            .into_iter()
            .filter_map(|item| {
                if item.url.is_empty() {
                    return None;
                }
                let datetime = NaiveDate::parse_from_str(&item.times, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc())
                    .unwrap_or_else(Utc::now);

                let disk_type = convert_disk_type(item.is_type);
                let password = item
                    .code
                    .filter(|c| !c.is_empty())
                    .or_else(|| extract_pwd_from_url(&item.url));

                let mut content = String::new();
                if !item.category.name.is_empty() {
                    content = format!("【{}】", item.category.name);
                }

                let mut tags = Vec::new();
                if !item.category.name.is_empty() {
                    tags.push(item.category.name);
                }

                Some(SearchResult {
                    message_id: format!("yunsou-{}", item.id),
                    unique_id: format!("yunsou-{}", item.id),
                    channel: String::new(),
                    channel_score: self.channel_score(),
                    datetime,
                    title: item.title,
                    content,
                    links: vec![Link {
                        disk_type,
                        url: item.url,
                        password: password.unwrap_or_default(),
                        datetime: None,
                        work_title: None,
                    }],
                    tags,
                    images: vec![],
                })
            })
            .collect();

        filter_results_by_keyword(&mut results, keyword);
        results
    }
}

fn convert_disk_type(is_type: i32) -> String {
    match is_type {
        0 => "quark".into(),
        1 => "aliyun".into(),
        2 => "baidu".into(),
        3 => "uc".into(),
        4 => "xunlei".into(),
        _ => "others".into(),
    }
}

fn extract_pwd_from_url(url: &str) -> Option<String> {
    let re = Regex::new(r"[?&]pwd=([0-9a-zA-Z]+)").unwrap();
    re.captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}
