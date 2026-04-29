use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::Utc;
use reqwest::Client;
use url::Url;

use crate::model::{CheckItem, CheckResponse, CheckResult};

#[derive(Clone)]
pub struct CheckService {
    cache: Arc<Mutex<HashMap<String, (CheckResult, Instant)>>>,
    client: Client,
}

impl Default for CheckService {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckService {
    pub fn new() -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 pansou-rust")
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            client,
        }
    }

    pub async fn check(&self, items: &[CheckItem]) -> CheckResponse {
        let mut results = Vec::with_capacity(items.len());
        for item in items {
            results.push(self.check_one(item));
        }
        CheckResponse { results }
    }

    fn check_one(&self, item: &CheckItem) -> CheckResult {
        let normalized = normalize_share_link(&item.disk_type, &item.url, &item.password);
        if normalized.is_empty() {
            return build_result(item, "", "uncertain", false, "链接格式无效");
        }
        let key = format!("{}|{}", item.disk_type, normalized);
        if let Ok(map) = self.cache.lock() {
            if let Some((cached, expires)) = map.get(&key) {
                if *expires > Instant::now() {
                    let mut hit = cached.clone();
                    hit.cache_hit = true;
                    return hit;
                }
            }
        }
        let mut result = quick_check(item, &normalized);
        result.cache_hit = false;
        let ttl = ttl_for_state(&result.state);
        if let Ok(mut map) = self.cache.lock() {
            map.insert(key, (result.clone(), Instant::now() + ttl));
        }
        result
    }
}

fn normalize_share_link(disk_type: &str, raw: &str, password: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return String::new();
    }
    let Ok(mut u) = Url::parse(raw) else {
        return raw.to_string();
    };
    u.set_fragment(None);
    if !password.is_empty() && (disk_type == "baidu" || disk_type == "quark" || disk_type == "uc") {
        let mut qp = u.query_pairs().into_owned().collect::<Vec<(String, String)>>();
        if !qp.iter().any(|(k, _)| k == "pwd") {
            qp.push(("pwd".into(), password.into()));
            let encoded = qp
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding(k), urlencoding(v)))
                .collect::<Vec<_>>()
                .join("&");
            u.set_query(Some(&encoded));
        }
    }
    u.to_string()
}

fn quick_check(item: &CheckItem, normalized: &str) -> CheckResult {
    match item.disk_type.as_str() {
        "aliyun" | "quark" | "uc" | "baidu" | "tianyi" | "123" | "xunlei" | "115" | "mobile" => {
            if normalized.contains("http") {
                build_result(item, normalized, "uncertain", false, "Rust版本检测器待接入平台API")
            } else {
                build_result(item, normalized, "bad", false, "链接失效")
            }
        }
        _ => build_result(item, normalized, "unsupported", false, "当前平台暂不支持检测"),
    }
}

fn build_result(item: &CheckItem, normalized: &str, state: &str, cache_hit: bool, summary: &str) -> CheckResult {
    let now = Utc::now().timestamp_millis();
    let exp = now + ttl_for_state(state).as_millis() as i64;
    CheckResult {
        disk_type: item.disk_type.clone(),
        url: item.url.clone(),
        normalized_url: Some(normalized.to_string()),
        state: state.to_string(),
        cache_hit,
        checked_at: now,
        expires_at: exp,
        summary: Some(summary.to_string()),
    }
}

fn ttl_for_state(state: &str) -> Duration {
    match state {
        "ok" => Duration::from_secs(24 * 3600),
        "bad" => Duration::from_secs(6 * 3600),
        "locked" => Duration::from_secs(12 * 3600),
        "unsupported" => Duration::from_secs(24 * 3600),
        _ => Duration::from_secs(30 * 60),
    }
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(disk_type: &str, url: &str) -> CheckItem {
        CheckItem { disk_type: disk_type.into(), url: url.into(), password: String::new() }
    }

    #[test]
    fn test_ttl_for_state_ok() {
        assert_eq!(ttl_for_state("ok"), Duration::from_secs(24 * 3600));
    }

    #[test]
    fn test_ttl_for_state_bad() {
        assert_eq!(ttl_for_state("bad"), Duration::from_secs(6 * 3600));
    }

    #[test]
    fn test_ttl_for_state_locked() {
        assert_eq!(ttl_for_state("locked"), Duration::from_secs(12 * 3600));
    }

    #[test]
    fn test_ttl_for_state_unsupported() {
        assert_eq!(ttl_for_state("unsupported"), Duration::from_secs(24 * 3600));
    }

    #[test]
    fn test_ttl_for_state_default() {
        assert_eq!(ttl_for_state("unknown"), Duration::from_secs(30 * 60));
    }

    #[test]
    fn test_normalize_empty_url() {
        assert_eq!(normalize_share_link("baidu", "", ""), "");
    }

    #[test]
    fn test_normalize_adds_pwd_for_baidu() {
        let result = normalize_share_link("baidu", "https://pan.baidu.com/s/abc", "testpwd");
        assert!(result.contains("pwd=testpwd"));
    }

    #[test]
    fn test_normalize_no_pwd_for_non_baidu_quark_uc() {
        let result = normalize_share_link("115", "https://115.com/s/abc", "testpwd");
        assert!(!result.contains("pwd="));
    }

    #[test]
    fn test_normalize_keeps_existing_pwd() {
        let result = normalize_share_link("baidu", "https://pan.baidu.com/s/abc?pwd=existing", "newpwd");
        assert!(result.contains("pwd=existing"));
        assert!(!result.contains("pwd=newpwd"));
    }

    #[test]
    fn test_normalize_removes_fragment() {
        let result = normalize_share_link("baidu", "https://pan.baidu.com/s/abc#section", "");
        assert!(!result.contains('#'));
    }

    #[test]
    fn test_quick_check_known_type() {
        let item = make_item("baidu", "https://pan.baidu.com/s/abc");
        let result = quick_check(&item, "https://pan.baidu.com/s/abc");
        assert_eq!(result.state, "uncertain");
    }

    #[test]
    fn test_quick_check_bad_link() {
        let item = make_item("baidu", "not-a-valid-url");
        let result = quick_check(&item, "not-a-valid-url");
        assert_eq!(result.state, "bad");
    }

    #[test]
    fn test_quick_check_unsupported() {
        let item = make_item("magnet", "magnet:?xt=urn:btih:abc");
        let result = quick_check(&item, "magnet:?xt=urn:btih:abc");
        assert_eq!(result.state, "unsupported");
    }

    #[test]
    fn test_check_empty_items() {
        let service = CheckService::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(service.check(&[]));
        assert!(resp.results.is_empty());
    }

    #[test]
    fn test_check_with_items() {
        let service = CheckService::new();
        let items = vec![make_item("baidu", "https://pan.baidu.com/s/abc")];
        let rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(service.check(&items));
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0].disk_type, "baidu");
        assert!(!resp.results[0].cache_hit);
    }

    #[test]
    fn test_check_cache_hit() {
        let service = CheckService::new();
        let items = vec![make_item("baidu", "https://pan.baidu.com/s/abc")];
        let rt = tokio::runtime::Runtime::new().unwrap();
        // First check — cache miss
        let resp1 = rt.block_on(service.check(&items));
        assert!(!resp1.results[0].cache_hit);
        // Second check — cache hit
        let resp2 = rt.block_on(service.check(&items));
        assert!(resp2.results[0].cache_hit);
    }

    #[test]
    fn test_check_expired_cache() {
        let service = CheckService::new();
        let items = vec![make_item("baidu", "https://pan.baidu.com/s/expired")];
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _resp1 = rt.block_on(service.check(&items));

        // Manually clear the cache to simulate expiry
        if let Ok(mut map) = service.cache.lock() {
            map.clear();
        }

        let resp2 = rt.block_on(service.check(&items));
        assert!(!resp2.results[0].cache_hit);
    }

    #[test]
    fn test_build_result_fields() {
        let item = make_item("quark", "https://pan.quark.cn/s/test");
        let result = build_result(&item, "https://pan.quark.cn/s/test", "ok", false, "valid");
        assert_eq!(result.disk_type, "quark");
        assert_eq!(result.url, "https://pan.quark.cn/s/test");
        assert_eq!(result.state, "ok");
        assert!(!result.cache_hit);
        assert_eq!(result.summary, Some("valid".into()));
        assert!(result.checked_at > 0);
        assert!(result.expires_at > result.checked_at);
        assert!(result.normalized_url.is_some());
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("abc123"), "abc123");
    }
}
