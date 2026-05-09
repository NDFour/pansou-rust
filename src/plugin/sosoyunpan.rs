use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use reqwest::Client;
use serde::Deserialize;

use tracing::debug;
use crate::model::{Link, SearchResult};

use super::{filter_results_by_keyword, SearchPlugin};

pub struct SosoyunpanPlugin;

const SEARCH_URL: &str = "https://www.sosoyunpan.com/list";

#[derive(Deserialize)]
struct SearchResponse {
    results: Vec<FileItem>,
    #[allow(dead_code)]
    #[serde(default)]
    total_count: u32,
}

#[derive(Deserialize)]
struct FileItem {
    #[serde(rename = "fileMd5")]
    file_md5: Option<String>,
    #[serde(rename = "fileTitle")]
    file_title: Option<String>,
    #[serde(rename = "fileLink")]
    file_link: Option<String>,
    #[serde(rename = "fileSize")]
    #[allow(dead_code)]
    file_size: Option<String>,
    #[serde(rename = "fileExt")]
    #[allow(dead_code)]
    file_ext: Option<String>,
    #[serde(rename = "shareDate")]
    share_date: Option<String>,
}

#[async_trait]
impl SearchPlugin for SosoyunpanPlugin {
    fn name(&self) -> &str {
        "sosoyunpan"
    }

    fn priority(&self) -> i32 {
        3
    }

    async fn search(&self, keyword: &str, client: &Client) -> Vec<SearchResult> {
        debug!(">>> {} 开始搜索, keyword: {}", self.name(), keyword);
        let body = serde_json::json!({
            "keyword": keyword,
            "source": "all",
            "formats": [],
            "year": "all",
            "sort": "relevance",
            "page": 1
        });

        let resp = match client
            .post(SEARCH_URL)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36")
            .header("Accept", "*/*")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Content-Type", "application/json")
            .header("Origin", "https://www.sosoyunpan.com")
            .header("Referer", "https://www.sosoyunpan.com")
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        debug!("<<< {} 响应 status code: {}", self.name(), resp.status());

        let resp_text = match resp.text().await {
            Ok(t) => t,
            Err(_) => return vec![],
        };

        let data: SearchResponse = match serde_json::from_str(&resp_text) {
            Ok(d) => d,
            Err(_) => return vec![],
        };

        debug!("<<< {} 开始解析 json, size: {}", self.name(), data.results.len());
        let mut results: Vec<SearchResult> = data
            .results
            .into_iter()
            .filter_map(|item| {
                let title = item.file_title.filter(|t| !t.is_empty())?;
                let title = title.replace("<em>", "").replace("</em>", "");
                let url = item.file_link.filter(|u| !u.is_empty())?;

                let datetime = item
                    .share_date
                    .as_deref()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc())
                    .unwrap_or_else(Utc::now);

                let unique_id = item
                    .file_md5
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| url.clone());

                let disk_type = link_type(&url);

                debug!("<<< {} 解析到链接: {}, disk_type: {}", self.name(), url, disk_type);

                Some(SearchResult {
                    message_id: format!("sosoyunpan-{}", unique_id),
                    unique_id: format!("sosoyunpan-{}", unique_id),
                    channel: self.name().to_string(),
                    channel_score: self.channel_score(),
                    datetime,
                    title,
                    content: String::new(),
                    links: vec![Link {
                        disk_type,
                        url,
                        password: String::new(),
                        datetime: None,
                        work_title: None,
                    }],
                    tags: vec![],
                    images: vec![],
                })
            })
            .collect();

        debug!("<<< {} 过滤关键词, size: {}", self.name(), results.len());
        filter_results_by_keyword(&mut results, keyword);
        debug!("<<< {} 过滤关键词后, size: {}", self.name(), results.len());
        results
    }
}

fn link_type(url: &str) -> String {
    let lower = url.to_lowercase();
    if lower.starts_with("magnet:") {
        return "magnet".into();
    }
    if lower.starts_with("ed2k://") {
        return "ed2k".into();
    }
    if lower.contains("pan.baidu.com") {
        return "baidu".into();
    }
    if lower.contains("pan.quark.cn") {
        return "quark".into();
    }
    if lower.contains("alipan.com") || lower.contains("aliyundrive.com") {
        return "aliyun".into();
    }
    if lower.contains("cloud.189.cn") {
        return "tianyi".into();
    }
    if lower.contains("drive.uc.cn") {
        return "uc".into();
    }
    if lower.contains("yun.139.com") || lower.contains("caiyun.139.com") {
        return "mobile".into();
    }
    if lower.contains("115.com") || lower.contains("115cdn.com") || lower.contains("anxia.com") {
        return "115".into();
    }
    if lower.contains("pan.xunlei.com") {
        return "xunlei".into();
    }
    if lower.contains("123pan.com") || lower.contains("123pan.cn") || lower.contains("123684.com") {
        return "123".into();
    }
    "others".into()
}
