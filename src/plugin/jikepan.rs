use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;

use crate::model::{Link, SearchResult};

use super::SearchPlugin;

pub struct JikepanPlugin;

const API_URL: &str = "https://api.jikepan.xyz/search";

#[derive(Deserialize)]
struct JikepanResponse {
    #[serde(default)]
    list: Vec<JikepanItem>,
}

#[derive(Deserialize)]
struct JikepanItem {
    name: String,
    #[serde(default)]
    links: Vec<JikepanLink>,
}

#[derive(Deserialize)]
struct JikepanLink {
    service: String,
    link: String,
    #[serde(default)]
    pwd: String,
}

#[async_trait]
impl SearchPlugin for JikepanPlugin {
    fn name(&self) -> &str {
        "jikepan"
    }

    async fn search(&self, keyword: &str, client: &Client) -> Vec<SearchResult> {
        let body = serde_json::json!({"name": keyword, "is_all": false});
        let resp = match client
            .post(API_URL)
            .header("Content-Type", "application/json")
            .header("Referer", "https://jikepan.xyz/")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        let data: JikepanResponse = match resp.json().await {
            Ok(d) => d,
            Err(_) => return vec![],
        };

        let now = Utc::now();
        data.list
            .into_iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if item.links.is_empty() {
                    return None;
                }
                let links: Vec<Link> = item
                    .links
                    .into_iter()
                    .filter_map(|l| {
                        let disk_type = convert_service(&l.service);
                        if disk_type.is_empty() {
                            return None;
                        }
                        Some(Link {
                            disk_type,
                            url: l.link,
                            password: l.pwd,
                            datetime: None,
                            work_title: None,
                        })
                    })
                    .collect();
                if links.is_empty() {
                    return None;
                }
                Some(SearchResult {
                    message_id: format!("jikepan_{}", i),
                    unique_id: format!("jikepan_{}", i),
                    channel: String::new(),
                    datetime: now,
                    title: item.name,
                    content: String::new(),
                    links,
                    tags: vec![],
                    images: vec![],
                })
            })
            .collect()
    }
}

fn convert_service(service: &str) -> String {
    match service.to_lowercase().as_str() {
        "baidu" => "baidu".into(),
        "aliyun" => "aliyun".into(),
        "xunlei" => "xunlei".into(),
        "quark" => "quark".into(),
        "189cloud" => "tianyi".into(),
        "115" => "115".into(),
        "123" => "123".into(),
        "pikpak" => "pikpak".into(),
        "caiyun" => "mobile".into(),
        "ed2k" => "ed2k".into(),
        "magnet" => "magnet".into(),
        "uc" => "uc".into(),
        "unknown" => String::new(),
        _ => "others".into(),
    }
}
