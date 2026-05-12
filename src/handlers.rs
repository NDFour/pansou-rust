use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::{
    model::{ApiResponse, CheckRequest, MetricRequest, SearchRequest, SearchResult},
    resource_cache::ResourceInfo,
    seo,
    AppState,
};

pub async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let resp = json!({
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "status":"ok",
        "plugins_enabled": true,
        "native_plugins": state.search_service.plugin_registry().list().into_iter().map(|p| p.name()).collect::<Vec<_>>(),
        "channels": state.config.channels,
    });
    (StatusCode::OK, Json(resp))
}

pub async fn search_get_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let req = build_request_from_query(&state, q);
    search_impl(state, req).await
}

pub async fn search_post_handler(
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<SearchRequest>,
) -> impl IntoResponse {
    if req.channels.is_empty() {
        req.channels = state.config.channels.clone();
    }
    normalize_search_request(&mut req);
    search_impl(state, req).await
}

async fn search_impl(state: Arc<AppState>, req: SearchRequest) -> impl IntoResponse {
    if req.keyword.trim().is_empty() {
        let err = crate::model::ApiErrorResponse {
            code: 400,
            message: "关键词不能为空".to_string(),
        };
        return (StatusCode::BAD_REQUEST, Json(json!(err)));
    }
    let result = state.search_service.search(&req).await;
    (StatusCode::OK, Json(json!(ApiResponse::success(result))))
}

pub async fn check_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CheckRequest>,
) -> impl IntoResponse {
    if req.items.is_empty() {
        let err = crate::model::ApiErrorResponse {
            code: 400,
            message: "items不能为空".to_string(),
        };
        return (StatusCode::BAD_REQUEST, Json(json!(err)));
    }
    let response = state.check_service.check(&req.items).await;
    (StatusCode::OK, Json(json!(response)))
}

pub async fn metric_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MetricRequest>,
) -> impl IntoResponse {
    match req.metric_type.as_str() {
        "click" => {
            if req.keyword.trim().is_empty() || req.title.trim().is_empty() || req.url.trim().is_empty() || req.channel.trim().is_empty() {
                let err = crate::model::ApiErrorResponse {
                    code: 400,
                    message: "keyword、title、url 不能为空".to_string(),
                };
                return (StatusCode::BAD_REQUEST, Json(json!(err)));
            }
            log_metric(&req);

            // 🆕 资源自动收录：从 click 事件提取资源信息
            let disk_type = classify_disk_type_from_url(&req.url);
            if disk_type != "others" {
                state.resource_cache.insert(
                    &req.title,
                    &req.url,
                    &disk_type,
                    &req.channel,
                    "",
                );
            }
        }
        _ => {
            warn!("无法识别的 metric_type: {}", req.metric_type);
        }
    }
    (StatusCode::OK, Json(json!({"code": 0, "message": "success"})))
}

fn log_metric(req: &MetricRequest) {
    info!("log_metric_info: {:?}", serde_json::to_string(req).unwrap());
}

fn classify_disk_type_from_url(url: &str) -> String {
    let lower = url.to_lowercase();
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

fn build_request_from_query(state: &Arc<AppState>, q: HashMap<String, String>) -> SearchRequest {
    let keyword = q.get("kw").cloned().unwrap_or_default();
    let channels = split_csv(q.get("channels"));
    let plugins = split_csv(q.get("plugins"));
    let cloud_types = split_csv(q.get("cloud_types"));
    let concurrency = q
        .get("conc")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or_default();
    let force_refresh = q.get("refresh").map(|v| v == "true").unwrap_or(false);
    let source_type = q.get("src").cloned().unwrap_or_else(|| "all".to_string());
    let ext = q
        .get("ext")
        .and_then(|v| serde_json::from_str::<HashMap<String, Value>>(v).ok())
        .unwrap_or_default();

    let mut req = SearchRequest {
        keyword,
        channels: if channels.is_empty() {
            state.config.channels.clone()
        } else {
            channels
        },
        concurrency,
        force_refresh,
        result_type: String::new(),
        source_type,
        plugins,
        ext,
        cloud_types
    };
    normalize_search_request(&mut req);
    req
}

fn split_csv(v: Option<&String>) -> Vec<String> {
    v.map(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

fn normalize_search_request(req: &mut SearchRequest) {
    if req.source_type.is_empty() {
        req.source_type = "all".to_string();
    }
    match req.source_type.as_str() {
        "tg" => req.plugins.clear(),
        "plugin" => req.channels.clear(),
        _ => {}
    }
}

pub async fn robots_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let domain = if state.config.domain.is_empty() {
        ""
    } else {
        &state.config.domain
    };
    let body = format!(
        "User-agent: *\n\
         Allow: /\n\
         Allow: /search\n\
         Allow: /resource/\n\
         Disallow: /api/\n\
         Sitemap: {}/sitemap.xml\n",
        domain
    );
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}

pub async fn sitemap_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let domain = &state.config.domain;
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>"#,
    );
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);

    // 首页
    xml.push_str(&format!(
        "<url><loc>{}/</loc><priority>1.0</priority><changefreq>daily</changefreq></url>",
        domain
    ));

    // 搜索页（使用固定热门关键词作为初始种子）
    let hot_keywords = [
        "流浪地球", "庆余年", "凡人修仙传", "三体", "哪吒", "封神",
        "鬼灭之刃", "海贼王", "火影忍者", "原神",
    ];
    for kw in &hot_keywords {
        let encoded = urlencoding(kw);
        xml.push_str(&format!(
            "<url><loc>{}/search?q={}</loc><priority>0.8</priority><changefreq>daily</changefreq></url>",
            domain, encoded
        ));
    }

    xml.push_str("</urlset>");

    (StatusCode::OK, [(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml)
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

pub async fn search_page_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let keyword = q.get("q").or(q.get("kw")).cloned().unwrap_or_default();
    let source_type = q.get("src").unwrap_or(&"all".to_string()).clone();
    let page: usize = q.get("page").and_then(|v| v.parse().ok()).unwrap_or(1);

    // 爬虫限流：通过 UA 检测，记录爬虫请求频率
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if seo::is_crawler(ua) {
        tracing::debug!("爬虫访问搜索页: keyword={}, ua={}", keyword, ua);
    }

    if keyword.trim().is_empty() {
        // 返回首页（重定向）
        return (
            StatusCode::FOUND,
            [(header::LOCATION, "/")],
            axum::body::Body::empty(),
        )
            .into_response();
    }

    // 构建搜索请求
    let req = SearchRequest {
        keyword: keyword.clone(),
        channels: state.config.channels.clone(),
        source_type: source_type.clone(),
        ..Default::default()
    };

    // 执行搜索
    let search_response = state.search_service.search(&req).await;

    // 截取当前页结果（每页 20 条）
    let page_size = 20;
    let start = (page.saturating_sub(1)) * page_size;
    let results: Vec<&SearchResult> = search_response.results.iter().skip(start).take(page_size).collect();

    // 构建 Tera 上下文
    let mut ctx = tera::Context::new();
    ctx.insert("keyword", &keyword);
    ctx.insert("source_type", &source_type);
    ctx.insert("page", &page);
    ctx.insert("total", &search_response.total);
    ctx.insert("results", &results);
    ctx.insert("domain", &format_domain(&state.config.domain));
    ctx.insert("related_searches", &seo::related_searches(&keyword));

    match seo::render_template(&state.templates, "search.html", ctx) {
        Ok(html) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "public, max-age=300"),
            ],
            html,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("模板渲染失败: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "500 Internal Server Error").into_response()
        }
    }
}

fn format_domain(domain: &str) -> String {
    if domain.is_empty() {
        String::new()
    } else {
        domain.trim_end_matches('/').to_string()
    }
}

pub async fn resource_page_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.resource_cache.get(&id) {
        Some(resource) => {
            let mut ctx = tera::Context::new();
            ctx.insert("resource", &resource);
            ctx.insert("domain", &format_domain(&state.config.domain));

            // 查找同一频道的相关资源
            let channel = resource.channel.clone();
            let current_id = resource.id.clone();
            let related: Vec<ResourceInfo> = state
                .resource_cache
                .all_ids()
                .iter()
                .filter_map(|rid| state.resource_cache.get(rid))
                .filter(|r| r.channel == channel && r.id != current_id)
                .take(5)
                .collect();
            ctx.insert("related_resources", &related);

            match seo::render_template(&state.templates, "resource.html", ctx) {
                Ok(html) => (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                        (header::CACHE_CONTROL, "public, max-age=3600"),
                    ],
                    html,
                )
                    .into_response(),
                Err(e) => {
                    tracing::error!("模板渲染失败: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "500 Internal Server Error").into_response()
                }
            }
        }
        None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_split_csv_basic() {
        let s = "a,b,c".to_string();
        let result = split_csv(Some(&s));
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_csv_with_spaces() {
        let s = "a, b ,c".to_string();
        let result = split_csv(Some(&s));
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_csv_empty_parts() {
        let s = "a,,b".to_string();
        let result = split_csv(Some(&s));
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn test_split_csv_none() {
        let result: Vec<String> = split_csv(None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_split_csv_empty_string() {
        let s = String::new();
        let result = split_csv(Some(&s));
        assert!(result.is_empty());
    }

    #[test]
    fn test_normalize_search_request_defaults() {
        let mut req = SearchRequest::default();
        normalize_search_request(&mut req);
        assert_eq!(req.source_type, "all");
    }

    #[test]
    fn test_normalize_search_request_tg_clears_plugins() {
        let mut req = SearchRequest {
            source_type: "tg".into(),
            plugins: vec!["p1".into()],
            ..Default::default()
        };
        normalize_search_request(&mut req);
        assert!(req.plugins.is_empty());
    }

    #[test]
    fn test_normalize_search_request_plugin_clears_channels() {
        let mut req = SearchRequest {
            source_type: "plugin".into(),
            channels: vec!["ch1".into()],
            ..Default::default()
        };
        normalize_search_request(&mut req);
        assert!(req.channels.is_empty());
    }

    #[test]
    fn test_build_request_from_query_basic() {
        use crate::config::AppConfig;
        use crate::service::SearchService;

        let config = std::sync::Arc::new(crate::AppState {
            config: AppConfig::default(),
            search_service: SearchService::new(2, Duration::from_secs(5 * 60), 512, ""),
            check_service: crate::service::CheckService::new(),
            templates: std::sync::Arc::new(tera::Tera::default()),
        });

        let mut q = HashMap::new();
        q.insert("kw".to_string(), "test_kw".to_string());
        let req = build_request_from_query(&config, q);
        assert_eq!(req.keyword, "test_kw");
    }

    #[test]
    fn test_build_request_from_query_with_channels() {
        use crate::config::AppConfig;
        use crate::service::SearchService;

        let config = std::sync::Arc::new(crate::AppState {
            config: AppConfig::default(),
            search_service: SearchService::new(2, Duration::from_secs(5 * 60), 512, ""),
            check_service: crate::service::CheckService::new(),
            templates: std::sync::Arc::new(tera::Tera::default()),
        });

        let mut q = HashMap::new();
        q.insert("kw".to_string(), "test".to_string());
        q.insert("channels".to_string(), "ch1,ch2".to_string());
        let req = build_request_from_query(&config, q);
        assert_eq!(req.channels, vec!["ch1", "ch2"]);
    }

    #[test]
    fn test_build_request_from_query_with_refresh() {
        use crate::config::AppConfig;
        use crate::service::SearchService;

        let config = std::sync::Arc::new(crate::AppState {
            config: AppConfig::default(),
            search_service: SearchService::new(2, Duration::from_secs(5 * 60), 512, ""),
            check_service: crate::service::CheckService::new(),
            templates: std::sync::Arc::new(tera::Tera::default()),
        });

        let mut q = HashMap::new();
        q.insert("kw".to_string(), "test".to_string());
        q.insert("refresh".to_string(), "true".to_string());
        let req = build_request_from_query(&config, q);
        assert!(req.force_refresh);
    }

}
