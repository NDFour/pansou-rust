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
        "status":"ok",
        "plugins_enabled": true,
        "native_plugins": 4,
        "channels_count": state.config.channels.len(),
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
    let mut result_type = q.get("res").cloned().unwrap_or_else(|| "merge".to_string());
    if result_type == "merge" {
        result_type = "merged_by_type".to_string();
    }
    let source_type = q.get("src").cloned().unwrap_or_else(|| "all".to_string());
    let ext = q
        .get("ext")
        .and_then(|v| serde_json::from_str::<HashMap<String, Value>>(v).ok())
        .unwrap_or_default();
    let filter = q
        .get("filter")
        .and_then(|v| serde_json::from_str::<crate::model::FilterConfig>(v).ok());

    let mut req = SearchRequest {
        keyword,
        channels: if channels.is_empty() {
            state.config.channels.clone()
        } else {
            channels
        },
        concurrency,
        force_refresh,
        result_type,
        source_type,
        plugins,
        ext,
        cloud_types,
        filter,
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
    if req.result_type.is_empty() || req.result_type == "merge" {
        req.result_type = "merged_by_type".to_string();
    }
    if req.source_type.is_empty() {
        req.source_type = "all".to_string();
    }
    match req.source_type.as_str() {
        "tg" => req.plugins.clear(),
        "plugin" => req.channels.clear(),
        _ => {}
    }
}
