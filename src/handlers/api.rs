use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use tracing::{info, warn};

use crate::{
    model::{ApiResponse, CheckRequest, MetricRequest, SearchRequest},
    AppState,
};

use super::{build_request_from_query, classify_disk_type_from_url, normalize_search_request};

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
