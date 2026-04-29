use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};

use crate::{
    model::{ApiResponse, CheckRequest, SearchRequest},
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
            search_service: SearchService::new(2, Duration::from_secs(5 * 60), 512),
            check_service: crate::service::CheckService::new(),
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
            search_service: SearchService::new(2, Duration::from_secs(5 * 60), 512),
            check_service: crate::service::CheckService::new(),
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
            search_service: SearchService::new(2, Duration::from_secs(5 * 60), 512),
            check_service: crate::service::CheckService::new(),
        });

        let mut q = HashMap::new();
        q.insert("kw".to_string(), "test".to_string());
        q.insert("refresh".to_string(), "true".to_string());
        let req = build_request_from_query(&config, q);
        assert!(req.force_refresh);
    }

}
