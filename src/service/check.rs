use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::Utc;
use reqwest::Client;
use url::Url;

use crate::model::{CheckItem, CheckRequest, CheckResponse, CheckResult};

#[derive(Clone)]
pub struct CheckService {
    cache: Arc<Mutex<HashMap<String, (CheckResult, Instant)>>>,
    go_compat_url: Option<String>,
    client: Client,
}

impl Default for CheckService {
    fn default() -> Self {
        Self::new(None)
    }
}

impl CheckService {
    pub fn new(go_compat_url: Option<String>) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 pansou-rust")
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            go_compat_url,
            client,
        }
    }

    pub async fn check(&self, items: &[CheckItem]) -> CheckResponse {
        if let Some(resp) = self.check_by_go_bridge(items).await {
            return resp;
        }
        let mut results = Vec::with_capacity(items.len());
        for item in items {
            results.push(self.check_one(item));
        }
        CheckResponse { results }
    }

    async fn check_by_go_bridge(&self, items: &[CheckItem]) -> Option<CheckResponse> {
        let base = self.go_compat_url.as_ref()?;
        let req = CheckRequest {
            items: items.to_vec(),
            view_token: String::new(),
        };
        let url = format!("{}/api/check/links", base.trim_end_matches('/'));
        let resp = self.client.post(url).json(&req).send().await.ok()?;
        resp.json::<CheckResponse>().await.ok()
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
