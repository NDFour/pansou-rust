use std::{cmp::Ordering, collections::{HashMap, HashSet}, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;

use crate::model::{
    MergedLink, MergedLinks, SearchRequest, SearchResponse, SearchResult,
};

use crate::plugin::PluginRegistry;

use tokio::task::JoinSet;

#[derive(Clone)]
pub struct SearchService {
    client: Client,
    concurrency: usize,
    plugin_registry: Arc<PluginRegistry>,
}

impl SearchService {
    pub fn new(concurrency: usize) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(12))
            .user_agent("Mozilla/5.0 pansou-rust")
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            concurrency: concurrency.max(1),
            plugin_registry: Arc::new(PluginRegistry::new()),
        }
    }

    pub fn plugin_registry(&self) -> Arc<PluginRegistry> {
        self.plugin_registry.clone()
    }

    pub async fn search(&self, req: &SearchRequest) -> SearchResponse {
        let source_type = if req.source_type.is_empty() { "all" } else { req.source_type.as_str() };
        let need_tg = source_type == "all" || source_type == "tg";
        let need_plugin = source_type == "all" || source_type == "plugin";

        let (tg_results, native_plugin_results) = tokio::join!(
            async { if need_tg { self.search_tg(req).await } else { vec![] } },
            async { if need_plugin { self.search_native_plugins(&req.keyword).await } else { vec![] } },
        );

        info!("合并搜索结果: tg: {:?}, plugin: {:?}", tg_results.len(), native_plugin_results.len());
        let mut all_results = merge_search_results(tg_results, native_plugin_results);
        sort_results_by_time_and_keywords(&mut all_results);

        let results_for_view = all_results.clone();
        let merged_by_type = merge_results_by_type(&all_results, &req.keyword, &req.cloud_types);
        let result_type = if req.result_type.is_empty() { "merged_by_type" } else { req.result_type.as_str() };
        match result_type {
            "all" => SearchResponse { total: results_for_view.len(), results: results_for_view, merged_by_type },
            "results" => SearchResponse { total: results_for_view.len(), results: results_for_view, merged_by_type: HashMap::new() },
            _ => {
                let total = merged_by_type.values().map(Vec::len).sum::<usize>();
                SearchResponse { total, results: vec![], merged_by_type }
            }
        }
    }

    async fn search_tg(&self, req: &SearchRequest) -> Vec<SearchResult> {
        // info!("搜索 tg: {:?}", req.keyword);
        let channels = if req.channels.is_empty() { vec![] } else { req.channels.clone() };
        if channels.is_empty() {
            return vec![];
        }

        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.concurrency));
        let keyword = req.keyword.clone();
        let client = self.client.clone();

        let mut set = JoinSet::new();
        for channel in channels {
            let client = client.clone();
            let kw = keyword.clone();
            let permit = semaphore.clone();
            set.spawn(async move {
                let _guard = permit.acquire().await;
                let url = format!("https://t.me/s/{}?q={}", channel, urlencoding(&kw));
                let resp = client.get(&url).send().await?;
                let body = resp.text().await?;
                Ok::<_, reqwest::Error>(parse_tg_results(&body, &channel))
            });
        }

        let mut out = Vec::new();
        while let Some(result) = set.join_next().await {
            if let Ok(Ok(results)) = result {
                out.extend(results);
            }
        }
        out
    }

    async fn search_native_plugins(&self, keyword: &str) -> Vec<SearchResult> {
        // info!("搜索插件: {:?}", keyword);
        let plugin_client = Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .unwrap_or_else(|_| Client::new());
        self.plugin_registry
            .search_all(keyword, &plugin_client)
            .await
    }

}

fn parse_tg_results(html: &str, channel: &str) -> Vec<SearchResult> {
    let doc = Html::parse_document(html);
    let (Ok(wrap_sel), Ok(msg_sel), Ok(date_sel), Ok(text_sel)) = (
        Selector::parse(".tgme_widget_message_wrap"),
        Selector::parse(".tgme_widget_message"),
        Selector::parse(".tgme_widget_message_date time"),
        Selector::parse(".tgme_widget_message_text"),
    ) else {
        return vec![];
    };

    let mut results = Vec::new();
    for wrap in doc.select(&wrap_sel) {
        let Some(msg) = wrap.select(&msg_sel).next() else { continue };
        let data_post = msg.value().attr("data-post").unwrap_or_default();
        let message_id = data_post.split('/').nth(1).unwrap_or_default();
        if message_id.is_empty() {
            continue;
        }

        let datetime = msg
            .select(&date_sel)
            .next()
            .and_then(|t| t.value().attr("datetime"))
            .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        let text = msg.select(&text_sel).next().map(|n| n.text().collect::<String>()).unwrap_or_default();
        let title = text.lines().next().unwrap_or_default().trim().to_string();
        let links = extract_links(&text);
        if links.is_empty() {
            continue;
        }
        results.push(SearchResult {
            message_id: message_id.to_string(),
            unique_id: format!("{}_{}", channel, message_id),
            channel: channel.to_string(),
            datetime,
            title,
            content: text,
            links,
            tags: vec![],
            images: vec![],
        });
    }
    results
}

fn extract_links(content: &str) -> Vec<crate::model::Link> {
    let Ok(re) = Regex::new(r#"https?://[^\s"'<>)]+"#) else { return vec![] };
    let mut seen = HashSet::new();
    let mut links = Vec::new();
    for m in re.find_iter(content) {
        let raw = m.as_str().trim().to_string();
        if !seen.insert(raw.clone()) {
            continue;
        }
        let disk_type = link_type(&raw);
        if disk_type == "others" {
            continue;
        }
        links.push(crate::model::Link {
            disk_type,
            url: raw,
            password: extract_pwd(content),
            datetime: None,
            work_title: None,
        });
    }
    links
}

fn extract_pwd(content: &str) -> String {
    for p in [r#"(?i)(?:提取码|访问码|pwd)[:：]?\s*([a-zA-Z0-9]{4,6})"#, r#"(?i)[?&]pwd=([a-zA-Z0-9]{4,6})"#] {
        if let Ok(re) = Regex::new(p) {
            if let Some(c) = re.captures(content) {
                if let Some(m) = c.get(1) {
                    return m.as_str().to_string();
                }
            }
        }
    }
    String::new()
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

/// 合并搜索结果，按照时间排序
fn merge_search_results(existing: Vec<SearchResult>, new_results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut map = HashMap::<String, SearchResult>::new();
    for r in existing.into_iter().chain(new_results.into_iter()) {
        let key = if !r.unique_id.is_empty() { r.unique_id.clone() } else if !r.message_id.is_empty() { r.message_id.clone() } else { format!("title_{}_{}", r.title, r.channel) };
        if let Some(old) = map.get(&key) {
            if completeness(&r) > completeness(old) {
                map.insert(key, r);
            }
        } else {
            map.insert(key, r);
        }
    }
    let mut merged = map.into_values().collect::<Vec<_>>();
    merged.sort_by(|a, b| b.datetime.cmp(&a.datetime));
    merged
}

/// 完整性得分 = 唯一标识得分 + 链接得分 + 内容得分 + 标签得分 + 标题得分
/// 信息越完整，得分越高
fn completeness(r: &SearchResult) -> usize {
    let mut score = 0;
    if !r.unique_id.is_empty() { score += 10; }
    score += r.links.len() * 2;
    if !r.content.is_empty() { score += 3; }
    score + r.tags.len() + (r.title.len() / 10)
}

fn sort_results_by_time_and_keywords(results: &mut [SearchResult]) {
    results.sort_by(|a, b| total_score(b).partial_cmp(&total_score(a)).unwrap_or(Ordering::Equal));
}

/// 总得分 = 时间得分 + 插件等级得分
fn total_score(r: &SearchResult) -> f64 {
    time_score(r.datetime) + plugin_level_score(plugin_level_from_result(r)) as f64
}

/// 时间得分 = 发布时间与当前时间的差值
/// 时间越近，得分越高
fn time_score(datetime: DateTime<Utc>) -> f64 {
    let diff_days = (Utc::now() - datetime).num_hours() as f64 / 24.0;
    if diff_days <= 1.0 { 500.0 } else if diff_days <= 3.0 { 400.0 } else if diff_days <= 7.0 { 300.0 } else if diff_days <= 30.0 { 200.0 } else if diff_days <= 90.0 { 100.0 } else if diff_days <= 365.0 { 50.0 } else { 20.0 }
}

/// 插件等级得分 = 1: tg, 2: 插件, 3: 其他(默认)
/// 插件等级越高，得分越高
fn plugin_level_score(level: i32) -> i32 {
    match level { 1 => 500, 2 => 1000, _ => 0 }
}

/// 插件等级 = 1: tg, 2: 插件, 3: 其他(默认)
/// 插件等级越高，得分越高
fn plugin_level_from_result(r: &SearchResult) -> i32 {
    if !r.channel.is_empty() { match r.channel.as_str() { "tg" => 1, "plugin" => 2, _ => 3 } } else { 3 }
}

/// 合并搜索结果 by 类型
fn merge_results_by_type(results: &[SearchResult], keyword: &str, cloud_types: &[String]) -> MergedLinks {
    let mut unique = HashMap::<String, MergedLink>::new();
    for r in results {
        for link in &r.links {
            let title = link.work_title.clone().unwrap_or_else(|| r.title.clone());
            let ml = MergedLink {
                url: link.url.clone(),
                password: link.password.clone(),
                note: title,
                datetime: link.datetime.unwrap_or(r.datetime),
                source: if !r.channel.is_empty() { Some(format!("{}", r.channel)) } else { Some("unknown".to_string()) },
                images: r.images.clone(),
            };
            match unique.get(&link.url) {
                Some(old) if old.datetime >= ml.datetime => {}
                _ => {
                    unique.insert(link.url.clone(), ml);
                }
            }
        }
    }

    let allow: HashSet<String> = cloud_types.iter().map(|s| s.to_lowercase()).collect();
    let mut out: MergedLinks = HashMap::new();
    for r in results {
        for link in &r.links {
            if let Some(ml) = unique.get(&link.url) {
                let t = link.disk_type.clone();
                if !allow.is_empty() && !allow.contains(&t.to_lowercase()) {
                    continue;
                }
                let bucket = out.entry(t).or_default();
                if !bucket.iter().any(|e| e.url == ml.url) {
                    bucket.push(ml.clone());
                }
            }
        }
    }
    out
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}
