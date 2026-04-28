use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;

use crate::model::{Link, SearchResult};

use super::SearchPlugin;

pub struct PanshushuPlugin;

const API_URL: &str = "https://www.panshushu.com/api/search";

#[derive(Deserialize)]
struct PanshushuResponse {
    code: i32,
    data: Option<PanshushuData>,
}

#[derive(Deserialize)]
struct PanshushuData {
    items: Vec<PanshushuItem>,
}

#[derive(Deserialize)]
struct PanshushuItem {
    id: i64,
    pwd: String,
    title: String,
    url: String,
}

#[async_trait]
impl SearchPlugin for PanshushuPlugin {
    fn name(&self) -> &str {
        "panshushu"
    }

    async fn search(&self, keyword: &str, client: &Client) -> Vec<SearchResult> {
        let url = format!(
            "{}?keyword={}&page=1&page_size=30&s=a1",
            API_URL,
            urlencoding(keyword)
        );
        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        let body: PanshushuResponse = match resp.json().await {
            Ok(b) => b,
            Err(_) => return vec![],
        };
        if body.code != 200 {
            return vec![];
        }
        let Some(data) = body.data else {
            return vec![];
        };

        let now = Utc::now();
        data.items
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let disk_type = link_type(&item.url);
                let title = item.title;
                SearchResult {
                    message_id: format!("panshushu_{}", item.id),
                    unique_id: format!("panshushu_{}", item.id),
                    channel: "panshushu".to_string(),
                    datetime: now - chrono::Duration::seconds(i as i64),
                    title: title.clone(),
                    content: title.clone(),
                    links: vec![Link {
                        disk_type,
                        url: item.url,
                        password: item.pwd,
                        datetime: Some(now - chrono::Duration::seconds(i as i64)),
                        work_title: Some(title),
                    }],
                    tags: vec![],
                    images: vec![],
                }
            })
            .collect()
    }
}

fn link_type(url: &str) -> String {
    let lower = url.to_lowercase();
    if lower.starts_with("magnet:") { return "magnet".into(); }
    if lower.starts_with("ed2k://") { return "ed2k".into(); }
    if lower.contains("pan.baidu.com") { return "baidu".into(); }
    if lower.contains("pan.quark.cn") { return "quark".into(); }
    if lower.contains("alipan.com") || lower.contains("aliyundrive.com") { return "aliyun".into(); }
    if lower.contains("cloud.189.cn") { return "tianyi".into(); }
    if lower.contains("drive.uc.cn") { return "uc".into(); }
    if lower.contains("yun.139.com") || lower.contains("caiyun.139.com") { return "mobile".into(); }
    if lower.contains("115.com") || lower.contains("115cdn.com") || lower.contains("anxia.com") { return "115".into(); }
    if lower.contains("pan.xunlei.com") { return "xunlei".into(); }
    if lower.contains("123pan.com") || lower.contains("123pan.cn") || lower.contains("123684.com") { return "123".into(); }
    "others".into()
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}
