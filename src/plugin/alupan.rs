use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::Semaphore;
use tracing::debug;

use crate::model::{Link, SearchResult};

use super::{filter_results_by_keyword, SearchPlugin};

pub struct AlupanPlugin;

const SEARCH_URL: &str = "https://www.aliupan.com";
const MAX_CONCURRENCY: usize = 12;
const DETAIL_TIMEOUT: Duration = Duration::from_secs(10);

#[async_trait]
impl SearchPlugin for AlupanPlugin {
    fn name(&self) -> &str {
        "alupan"
    }

    fn priority(&self) -> i32 {
        2
    }

    async fn search(&self, keyword: &str, client: &Client) -> Vec<SearchResult> {
        #[derive(Clone)]
        struct Article {
            title: String,
            detail_url: String,
            article_id: String,
            summary: String,
            publish_time: DateTime<Utc>,
        }

        let articles = {
            let search_url = format!("{}/?s={}", SEARCH_URL, urlencoding(keyword));
            let resp = match client
                .get(&search_url)
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
                .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
                .header("Referer", SEARCH_URL)
                .send()
                .await
            {
                Ok(r) => r,
                Err(_) => return vec![],
            };
            debug!("{} 请求到html: {}", self.name(), resp.status());

            let body = match resp.text().await {
                Ok(b) => b,
                Err(_) => return vec![],
            };

            let doc = Html::parse_document(&body);
            debug!("{} 解析到html: {}", self.name(), body);

            let article_sel = match Selector::parse("article.excerpt") {
                Ok(s) => s,
                Err(_) => return vec![],
            };
            let title_sel = match Selector::parse("header h2 a") {
                Ok(s) => s,
                Err(_) => return vec![],
            };
            let note_sel = match Selector::parse("p.note") {
                Ok(s) => s,
                Err(_) => return vec![],
            };

            let article_id_re = Regex::new(r"\?p=(\d+)").unwrap();
            let mut articles = Vec::new();

            for item in doc.select(&article_sel) {
                let Some(title_el) = item.select(&title_sel).next() else {
                    continue;
                };
                let title = title_el.text().collect::<String>().trim().to_string();
                let Some(detail_url) = title_el.value().attr("href") else {
                    continue;
                };
                let article_id = article_id_re
                    .captures(detail_url)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                if article_id.is_empty() {
                    continue;
                }

                let summary = item
                    .select(&note_sel)
                    .next()
                    .map(|n| n.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();

                let publish_time = Utc::now();

                articles.push(Article {
                    title,
                    detail_url: detail_url.to_string(),
                    article_id,
                    summary,
                    publish_time,
                });
            }
            // doc dropped here — scraper::Html is not Send
            articles
        };

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));
        let detail_client = build_detail_client();
        let mut handles = Vec::new();

        for art in &articles {
            let client = detail_client.clone();
            let sem = Arc::clone(&semaphore);
            let url = art.detail_url.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await;
                fetch_detail_links(&client, &url).await
            }));
        }

        let mut results = Vec::new();
        for (i, handle) in handles.into_iter().enumerate() {
            if let Ok(links) = handle.await {
                if links.is_empty() {
                    continue;
                }
                let art = &articles[i];
                results.push(SearchResult {
                    message_id: format!("alupan-{}", art.article_id),
                    unique_id: format!("alupan-{}", art.article_id),
                    channel: String::new(),
                    channel_score: self.channel_score(),
                    datetime: art.publish_time,
                    title: art.title.clone(),
                    content: art.summary.clone(),
                    links,
                    tags: vec![],
                    images: vec![],
                });
            }
        }

        filter_results_by_keyword(&mut results, keyword);
        results
    }
}

fn build_detail_client() -> Client {
    Client::builder()
        .connect_timeout(DETAIL_TIMEOUT)
        .timeout(DETAIL_TIMEOUT)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .unwrap_or_else(|_| Client::new())
}

async fn fetch_detail_links(client: &Client, detail_url: &str) -> Vec<Link> {
    let resp = match client
        .get(detail_url)
        .header("Referer", SEARCH_URL)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let body = match resp.text().await {
        Ok(b) => b,
        Err(_) => return vec![],
    };

    let doc = Html::parse_document(&body);
    let content_sel = match Selector::parse(".article-content a[href]") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let mut seen: HashMap<String, ()> = HashMap::new();
    let mut links = Vec::new();

    let link_patterns: Vec<(Regex, &str)> = vec![
        (Regex::new(r"https?://pan\.quark\.cn/s/[0-9A-Za-z]+").unwrap(), "quark"),
        (Regex::new(r"https?://www\.aliyundrive\.com/s/[0-9A-Za-z]+").unwrap(), "aliyun"),
        (
            Regex::new(r"https?://www\.aliyundrive\.com/drive/folder/[0-9A-Za-z]+").unwrap(),
            "aliyun",
        ),
    ];

    let pwd_re = Regex::new(r"(?:提取码|密码)[:：]?\s*([0-9A-Za-z]+)").unwrap();

    for node in doc.select(&content_sel) {
        let Some(href) = node.value().attr("href") else {
            continue;
        };
        let href = href.trim();
        if href.is_empty() {
            continue;
        }

        let (disk_type, normalized_url) = classify_link(href, &link_patterns);
        if disk_type.is_empty() {
            continue;
        }

        if seen.contains_key(&normalized_url) {
            continue;
        }

        let password = extract_password(&node, &pwd_re);
        links.push(Link {
            disk_type,
            url: normalized_url.clone(),
            password,
            datetime: None,
            work_title: None,
        });
        seen.insert(normalized_url, ());
    }

    links
}

fn classify_link(raw: &str, patterns: &[(Regex, &str)]) -> (String, String) {
    for (re, typ) in patterns {
        if let Some(m) = re.find(raw) {
            return (typ.to_string(), m.as_str().to_string());
        }
    }
    (String::new(), String::new())
}

fn extract_password(node: &scraper::ElementRef, pwd_re: &Regex) -> String {
    let text = node.text().collect::<String>();
    if let Some(caps) = pwd_re.captures(&text) {
        if let Some(m) = caps.get(1) {
            return m.as_str().trim().to_string();
        }
    }
    String::new()
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}
