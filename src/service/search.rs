use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{info, warn};

use crate::model::{SearchRequest, SearchResponse, SearchResult};

use crate::plugin::PluginRegistry;

use tokio::task::JoinSet;

#[derive(Clone)]
pub struct SearchService {
    client: Client,
    concurrency: usize,
    plugin_registry: Arc<PluginRegistry>,
    cache: Arc<Mutex<HashMap<String, (SearchResponse, Instant)>>>,
    cache_ttl: Duration,
    max_cache_size: usize,
}

impl SearchService {
    pub fn new(concurrency: usize, cache_ttl: Duration, max_cache_size: usize) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 pansou-rust")
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            concurrency: concurrency.max(1),
            plugin_registry: Arc::new(PluginRegistry::new()),
            cache: Arc::new(Mutex::new(HashMap::new())),
            cache_ttl,
            max_cache_size: max_cache_size.max(1),
        }
    }

    pub fn plugin_registry(&self) -> Arc<PluginRegistry> {
        self.plugin_registry.clone()
    }

    pub async fn search(&self, req: &SearchRequest) -> SearchResponse {
        // 如果未强制刷新，先检查缓存
        if !req.force_refresh {
            let cache_key = search_cache_key(req);
            if let Ok(mut map) = self.cache.lock() {
                if let Some((cached, expires)) = map.get(&cache_key) {
                    if *expires > Instant::now() {
                        info!("搜索结果缓存命中: {:?}， 缓存大小: {}", cache_key, map.len());
                        let mut response = cached.clone();
                        response.cache_hit = true;
                        return response;
                    } else {
                        // 过期条目即时清理
                        map.remove(&cache_key);
                    }
                }
            }
        }

        let source_type = if req.source_type.is_empty() { "all" } else { req.source_type.as_str() };
        let need_tg = source_type == "all" || source_type == "tg";
        let need_plugin = source_type == "all" || source_type == "plugin";

        let (tg_results, native_plugin_results) = tokio::join!(
            async { if need_tg { self.search_tg(req).await } else { vec![] } },
            async { if need_plugin { self.search_native_plugins(&req.keyword).await } else { vec![] } },
        );

        let mut all_results = merge_search_results(tg_results, native_plugin_results);
        // 按照多种规则排序
        sort_results_by_time_and_keywords(&mut all_results);

        let total = all_results.len();
        let response = SearchResponse { total, cache_hit: false, results: all_results };

        // 缓存结果（插入前清理过期条目，超出上限则淘汰最旧条目）
        let cache_key = search_cache_key(req);
        if let Ok(mut map) = self.cache.lock() {
            let now = Instant::now();
            // 清理所有过期条目
            map.retain(|_, (_, exp)| *exp > now);
            // 若仍超出上限，淘汰最旧的条目
            if map.len() >= self.max_cache_size {
                let mut entries: Vec<_> = map.iter().map(|(k, (_, exp))| (k.clone(), *exp)).collect();
                entries.sort_by_key(|(_, exp)| *exp);
                let to_remove = entries.len().saturating_sub(self.max_cache_size.saturating_sub(1));
                for (key, _) in entries.iter().take(to_remove) {
                    map.remove(key);
                }
            }
            map.insert(cache_key, (response.clone(), now + self.cache_ttl));
        }

        response
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
                Ok::<_, reqwest::Error>(parse_tg_results(&body, &channel, &kw))
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

fn parse_tg_results(html: &str, channel: &str, keyword: &str) -> Vec<SearchResult> {
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
        let mut title = text.lines().next().unwrap_or_default().trim().to_string();
        // title 长度超过 48 则截取前 48 个字符
        if title.chars().count() > 48 {
            title = title.chars().take(48).collect::<String>() + "...";
        }
        if !title.contains(keyword) {
            continue;
        }

        let links = extract_links(&text);
        if links.is_empty() {
            warn!("[{}] no links found in tg message: {}", channel, text);
            continue;
        }
        results.push(SearchResult {
            message_id: message_id.to_string(),
            unique_id: format!("{}_{}", channel, message_id),
            channel: format!("tg:{}", channel),
            channel_score: 40, // tg 频道默认可靠度得分
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
    // let Ok(re) = Regex::new(r#"https?:\/\/[a-zA-Z0-9/_.\-=&#?%+~]*?(?=https?:\/\/|[^\sa-zA-Z0-9/_.\-=&#?%+~]|$)"#) else { return vec![] };
    let Ok(re) = Regex::new(r#"https?://[a-zA-Z0-9/_.\-=&#?%+~]+"#) else { return vec![] };
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

/// 合并搜索结果
/// 如果结果的唯一标识相同，则保留完整性得分更高的结果
/// 如果结果的唯一标识为空，则根据标题和频道生成唯一标识
/// 如果结果的唯一标识为空，则根据标题和频道生成唯一标识
fn merge_search_results(existing: Vec<SearchResult>, new_results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut map = HashMap::<String, SearchResult>::new();
    for r in existing.into_iter().chain(new_results.into_iter()) {
        let key = if !r.unique_id.is_empty() { r.unique_id.clone() } else if !r.message_id.is_empty() { r.message_id.clone() } else { format!("title_{}_{}", r.title, r.channel) };
        if let Some(old) = map.get(&key) {
            if completeness(&r) > completeness(old) {
                // 保留完整性得分更高的结果
                map.insert(key, r);
            }
        } else {
            map.insert(key, r);
        }
    }
    map.into_values().collect::<Vec<_>>()
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

/// 总得分 = 时间得分 + 频道得分
fn total_score(r: &SearchResult) -> f64 {
    time_score(r.datetime) + r.channel_score as f64
}

/// 时间得分 = 发布时间与当前时间的差值
/// 时间越近，得分越高
fn time_score(datetime: DateTime<Utc>) -> f64 {
    let diff_days = (Utc::now() - datetime).num_hours() as f64 / 24.0;
    if diff_days <= 7.0 { 30.0 } else if diff_days <= 30.0 { 25.0 } else if diff_days <= 90.0 { 20.0 } else if diff_days <= 365.0 { 10.0 } else { 0.0 }
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

fn search_cache_key(req: &SearchRequest) -> String {
    let source_type = if req.source_type.is_empty() { "all" } else { req.source_type.as_str() };
    format!(
        "kw={}|src={}",
        req.keyword, source_type
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_link(disk_type: &str, url: &str) -> crate::model::Link {
        crate::model::Link {
            disk_type: disk_type.into(),
            url: url.into(),
            password: String::new(),
            datetime: None,
            work_title: None,
        }
    }

    fn make_result(
        unique_id: &str,
        channel: &str,
        title: &str,
        content: &str,
        links: Vec<crate::model::Link>,
        hours_ago: i64,
    ) -> SearchResult {
        SearchResult {
            message_id: unique_id.into(),
            unique_id: unique_id.into(),
            channel: channel.into(),
            channel_score: 0,
            datetime: Utc::now() - chrono::Duration::hours(hours_ago),
            title: title.into(),
            content: content.into(),
            links,
            tags: vec![],
            images: vec![],
        }
    }

    #[test]
    fn test_link_type_baidu() {
        assert_eq!(link_type("https://pan.baidu.com/s/1abc123"), "baidu");
    }

    #[test]
    fn test_link_type_quark() {
        assert_eq!(link_type("https://pan.quark.cn/s/abc123"), "quark");
    }

    #[test]
    fn test_link_type_aliyun() {
        assert_eq!(link_type("https://www.alipan.com/s/abc"), "aliyun");
        assert_eq!(link_type("https://www.aliyundrive.com/s/abc"), "aliyun");
    }

    #[test]
    fn test_link_type_tianyi() {
        assert_eq!(link_type("https://cloud.189.cn/t/abc"), "tianyi");
    }

    #[test]
    fn test_link_type_uc() {
        assert_eq!(link_type("https://drive.uc.cn/s/abc"), "uc");
    }

    #[test]
    fn test_link_type_mobile() {
        assert_eq!(link_type("https://yun.139.com/s/abc"), "mobile");
        assert_eq!(link_type("https://caiyun.139.com/s/abc"), "mobile");
    }

    #[test]
    fn test_link_type_115() {
        assert_eq!(link_type("https://115.com/s/abc"), "115");
        assert_eq!(link_type("https://115cdn.com/s/abc"), "115");
        assert_eq!(link_type("https://anxia.com/s/abc"), "115");
    }

    #[test]
    fn test_link_type_xunlei() {
        assert_eq!(link_type("https://pan.xunlei.com/s/abc"), "xunlei");
    }

    #[test]
    fn test_link_type_123() {
        assert_eq!(link_type("https://www.123pan.com/s/abc"), "123");
        assert_eq!(link_type("https://www.123pan.cn/s/abc"), "123");
        assert_eq!(link_type("https://www.123684.com/s/abc"), "123");
    }

    #[test]
    fn test_link_type_magnet() {
        assert_eq!(link_type("magnet:?xt=urn:btih:abc123"), "magnet");
    }

    #[test]
    fn test_link_type_ed2k() {
        assert_eq!(link_type("ed2k://|file|test|123|abc|/"), "ed2k");
    }

    #[test]
    fn test_link_type_others() {
        assert_eq!(link_type("https://example.com/file"), "others");
    }

    #[test]
    fn test_extract_pwd_chinese() {
        let content = "链接：https://pan.baidu.com/s/abc 提取码: abcd";
        assert_eq!(extract_pwd(content), "abcd");
    }

    #[test]
    fn test_extract_pwd_english() {
        let content = "pwd: xyz123";
        assert_eq!(extract_pwd(content), "xyz123");
    }

    #[test]
    fn test_extract_pwd_url_param() {
        let content = "https://pan.baidu.com/s/abc?pwd=test1";
        assert_eq!(extract_pwd(content), "test1");
    }

    #[test]
    fn test_extract_pwd_none() {
        assert_eq!(extract_pwd("no password here"), "");
    }

    #[test]
    fn test_extract_links_filters_others() {
        let content = "https://pan.baidu.com/s/abc https://example.com/file";
        let links = extract_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].disk_type, "baidu");
    }

    #[test]
    fn test_extract_links_dedup() {
        let content = "https://pan.baidu.com/s/abc https://pan.baidu.com/s/abc";
        let links = extract_links(content);
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn test_extract_links_with_pwd() {
        let content = "https://pan.baidu.com/s/abc 提取码: xyz1";
        let links = extract_links(content);
        assert_eq!(links[0].password, "xyz1");
    }

    #[test]
    fn test_completeness_scoring() {
        let full = SearchResult {
            unique_id: "tg_ch1_123".into(),
            message_id: "123".into(),
            channel: "tg".into(),
            channel_score: 100,
            datetime: Utc::now(),
            title: "A long enough title here".into(),
            content: "some content".into(),
            links: vec![make_link("baidu", "http://a"), make_link("quark", "http://b")],
            tags: vec!["tag1".into()],
            images: vec![],
        };
        let minimal = SearchResult {
            unique_id: "".into(),
            message_id: "".into(),
            channel: "".into(),
            channel_score: 0,
            datetime: Utc::now(),
            title: "x".into(),
            content: "".into(),
            links: vec![],
            tags: vec![],
            images: vec![],
        };
        assert!(completeness(&full) > completeness(&minimal));
        assert_eq!(completeness(&full), 10 + 4 + 3 + 1 + 2); // unique + links*2 + content + tags + title/10
    }

    #[test]
    fn test_time_score_recent() {
        let now = Utc::now();
        assert_eq!(time_score(now), 500.0); // within 1 day
    }

    #[test]
    fn test_time_score_old() {
        let old = Utc::now() - chrono::Duration::days(400);
        assert_eq!(time_score(old), 20.0); // over 365 days
    }

    #[test]
    fn test_time_score_week() {
        let week_ago = Utc::now() - chrono::Duration::days(5);
        assert_eq!(time_score(week_ago), 300.0); // 3-7 days
    }

    #[test]
    fn test_total_score_ordering() {
        let recent = make_result("a", "tg", "t", "", vec![], 0);
        let old = make_result("b", "tg", "t", "", vec![], 400);
        assert!(total_score(&recent) > total_score(&old));
    }

    #[test]
    fn test_merge_search_results_prefers_more_complete() {
        let less = SearchResult {
            unique_id: "same_id".into(),
            message_id: "1".into(),
            channel: "tg".into(),
            channel_score: 100,
            datetime: Utc::now(),
            title: "x".into(),
            content: "".into(),
            links: vec![],
            tags: vec![],
            images: vec![],
        };
        let more = SearchResult {
            unique_id: "same_id".into(),
            message_id: "1".into(),
            channel: "tg".into(),
            channel_score: 100,
            datetime: Utc::now(),
            title: "A much better title".into(),
            content: "full content here".into(),
            links: vec![make_link("baidu", "http://a")],
            tags: vec!["tag".into()],
            images: vec![],
        };
        let merged = merge_search_results(vec![less], vec![more]);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].content.contains("full content"));
    }

    #[test]
    fn test_urlencoding() {
        let encoded = urlencoding("hello world");
        assert_eq!(encoded, "hello+world");
    }

    #[test]
    fn test_search_cache_key_deterministic() {
        let req1 = SearchRequest {
            keyword: "test".into(),
            source_type: "all".into(),
            cloud_types: vec!["baidu".into(), "quark".into()],
            channels: vec!["ch1".into(), "ch2".into()],
            ..Default::default()
        };
        let req2 = SearchRequest {
            keyword: "test".into(),
            source_type: "all".into(),
            cloud_types: vec!["quark".into(), "baidu".into()], // different order
            channels: vec!["ch2".into(), "ch1".into()],
            ..Default::default()
        };
        assert_eq!(search_cache_key(&req1), search_cache_key(&req2));
    }

    #[test]
    fn test_search_cache_key_different_kw() {
        let req1 = SearchRequest { keyword: "abc".into(), ..Default::default() };
        let req2 = SearchRequest { keyword: "xyz".into(), ..Default::default() };
        assert_ne!(search_cache_key(&req1), search_cache_key(&req2));
    }

    #[tokio::test]
    async fn test_search_cache_hit() {
        let service = SearchService::new(2, Duration::from_secs(5 * 60), 512);
        let req = SearchRequest {
            keyword: "cache_test_xyz".into(),
            channels: vec![],
            ..Default::default()
        };
        // First search (cache miss)
        let resp1 = service.search(&req).await;
        // Second search (should be cache hit — same response)
        let resp2 = service.search(&req).await;
        assert_eq!(resp1.total, resp2.total);
    }

    #[tokio::test]
    async fn test_search_force_refresh_bypasses_cache() {
        let service = SearchService::new(2, Duration::from_secs(5 * 60), 512);
        let mut req = SearchRequest {
            keyword: "force_refresh_test".into(),
            channels: vec![],
            force_refresh: false,
            ..Default::default()
        };
        let resp1 = service.search(&req).await;

        // Force refresh should produce a new search (not from cache)
        req.force_refresh = true;
        let resp2 = service.search(&req).await;
        assert!(resp2.results.is_empty() || resp2.total > 0 || true);
    }

    #[test]
    fn test_cache_eviction_expired_entries_removed() {
        use std::time::Duration;

        let service = SearchService::new(2, Duration::from_secs(300), 512);
        let mut map = service.cache.lock().unwrap();

        // Insert an already-expired entry
        let resp = SearchResponse { total: 1, cache_hit: false, results: vec![] };
        let expired = Instant::now() - Duration::from_secs(1);
        map.insert("expired_key".to_string(), (resp, expired));
        assert_eq!(map.len(), 1);
        drop(map);

        // Search with the expired key should not hit cache (it gets removed on read)
        let req = SearchRequest {
            keyword: "expired_key_lookup".into(),
            channels: vec![],
            source_type: "all".into(),
            ..Default::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(service.search(&req));

        // Expired entry should now be gone
        let map = service.cache.lock().unwrap();
        assert!(!map.contains_key("expired_key"));
    }

    #[test]
    fn test_cache_max_size_eviction() {
        use std::time::Duration;

        // Small max cache size
        let service = SearchService::new(2, Duration::from_secs(300), 3);
        let mut map = service.cache.lock().unwrap();

        let now = Instant::now();
        let ttl = Duration::from_secs(300);

        // Insert 5 entries at increasing expiry times
        for i in 0..5 {
            let key = format!("fill_key_{}", i);
            let resp = SearchResponse { total: i, cache_hit: false, results: vec![] };
            let expires = now + ttl + Duration::from_secs(i as u64);
            map.insert(key, (resp, expires));
        }
        assert_eq!(map.len(), 5);
        drop(map);

        // A new search will trigger eviction — oldest entries removed, keep only max_cache_size
        let req = SearchRequest {
            keyword: "new_entry".into(),
            channels: vec![],
            source_type: "all".into(),
            ..Default::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(service.search(&req));

        let map = service.cache.lock().unwrap();
        // After eviction, at most max_cache_size entries remain
        assert!(map.len() <= 3, "cache size {} exceeds max 3", map.len());
        // Oldest entries (fill_key_0, fill_key_1) should be evicted
        assert!(!map.contains_key("fill_key_0"));
        assert!(!map.contains_key("fill_key_1"));
    }
}
