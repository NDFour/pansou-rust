use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};

use crate::{
    auth::generate_token,
    model::{ApiResponse, CheckRequest, LoginRequest, LoginResponse, SearchRequest},
    AppState,
};

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    if req.username.trim().is_empty() || req.password.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"参数错误：用户名和密码不能为空"})),
        );
    }
    if !state.config.auth_enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error":"认证功能未启用"})),
        );
    }
    if state.config.auth_users.is_empty() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"认证系统未正确配置"})),
        );
    }
    let matched = state
        .config
        .auth_users
        .get(&req.username)
        .map(|pwd| pwd == &req.password)
        .unwrap_or(false);
    if !matched {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"用户名或密码错误"})),
        );
    }

    match generate_token(
        &req.username,
        &state.config.auth_jwt_secret,
        state.config.auth_token_expiry_hours,
    ) {
        Ok((token, expires_at)) => (
            StatusCode::OK,
            Json(json!(LoginResponse {
                token,
                expires_at,
                username: req.username,
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"生成令牌失败"})),
        ),
    }
}

pub async fn verify_handler(
    State(state): State<Arc<AppState>>,
    username: Option<Extension<String>>,
) -> impl IntoResponse {
    if !state.config.auth_enabled {
        return (
            StatusCode::OK,
            Json(json!({"valid":true,"message":"认证功能未启用"})),
        );
    }
    if let Some(Extension(name)) = username {
        return (StatusCode::OK, Json(json!({"valid":true,"username":name})));
    }
    (StatusCode::UNAUTHORIZED, Json(json!({"error":"未授权"})))
}

pub async fn logout_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"message":"退出成功"})))
}

pub async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let resp = json!({
        "status":"ok",
        "auth_enabled": state.config.auth_enabled,
        "plugins_enabled": true,
        "native_plugins": 4,
        "go_compat_enabled": state.config.go_compat_url.is_some(),
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
