use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Serialize;

use crate::{
    model::SearchRequest,
    resource_cache::ResourceInfo,
    templates,
    AppState,
};

use super::format_domain;

#[derive(Debug, Clone, Serialize)]
struct LinkItem {
    url: String,
    password: String,
    title: String,
    source: String,
    datetime: String,
    disk_type: String,
}

#[derive(Debug, Clone, Serialize)]
struct TypeGroup {
    disk_type: String,
    label: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct TypeGroupWithLinks {
    disk_type: String,
    label: String,
    links: Vec<LinkItem>,
}

fn type_friendly(dt: &str) -> String {
    match dt {
        "baidu" => "百度网盘".into(),
        "quark" => "夸克网盘".into(),
        "aliyun" => "阿里云盘".into(),
        "tianyi" => "天翼云盘".into(),
        "xunlei" => "迅雷云盘".into(),
        "115" => "115网盘".into(),
        "uc" => "UC网盘".into(),
        "123" => "123云盘".into(),
        "mobile" => "移动云盘".into(),
        "magnet" => "磁力链接".into(),
        "ed2k" => "电驴链接".into(),
        _ => dt.to_string(),
    }
}

fn format_datetime(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

pub async fn search_page_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let keyword = q.get("kw").unwrap_or(&String::new()).clone();
    if keyword.is_empty() {
        return (
            StatusCode::FOUND,
            [(header::LOCATION, "/")],
            axum::body::Body::empty(),
        )
            .into_response();
    }

    let source_type = q.get("src").unwrap_or(&"all".to_string()).clone();
    let page: usize = q.get("page").and_then(|v| v.parse().ok()).unwrap_or(1);

    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if templates::is_crawler(ua) {
        tracing::info!("爬虫访问搜索页: keyword={}, ua={}", keyword, ua);
    }

    if keyword.trim().is_empty() {
        return (
            StatusCode::FOUND,
            [(header::LOCATION, "/")],
            axum::body::Body::empty(),
        )
            .into_response();
    }

    let req = SearchRequest {
        keyword: keyword.clone(),
        channels: state.config.channels.clone(),
        source_type: source_type.clone(),
        ..Default::default()
    };

    let search_response = state.search_service.search(&req).await;

    let disk_type_filter = q.get("disk_type").cloned().unwrap_or_else(|| "__all__".to_string());
    let mut seen = std::collections::HashSet::new();
    let mut all_links: Vec<LinkItem> = Vec::new();

    for result in &search_response.results {
        for link in &result.links {
            if seen.insert(link.url.clone()) {
                all_links.push(LinkItem {
                    url: link.url.clone(),
                    password: link.password.clone(),
                    title: result.title.clone(),
                    source: result.channel.clone(),
                    datetime: format_datetime(result.datetime),
                    disk_type: link.disk_type.clone(),
                });
            }
        }
    }

    let mut groups: HashMap<String, Vec<&LinkItem>> = HashMap::new();
    for item in &all_links {
        groups.entry(item.disk_type.clone()).or_default().push(item);
    }

    let type_order = ["baidu", "quark", "aliyun", "tianyi", "xunlei", "115", "uc", "123", "mobile", "magnet", "ed2k"];
    let mut type_groups: Vec<TypeGroup> = Vec::new();
    for dt in &type_order {
        if let Some(links) = groups.get(*dt) {
            type_groups.push(TypeGroup {
                disk_type: dt.to_string(),
                label: type_friendly(dt),
                count: links.len(),
            });
        }
    }
    for (dt, links) in &groups {
        if !type_order.contains(&dt.as_str()) {
            type_groups.push(TypeGroup {
                disk_type: dt.clone(),
                label: type_friendly(dt),
                count: links.len(),
            });
        }
    }

    let total_links = all_links.len();

    let page_size = 20;
    let filtered: Vec<&LinkItem> = if disk_type_filter == "__all__" {
        all_links.iter().collect()
    } else {
        all_links.iter().filter(|l| l.disk_type == disk_type_filter).collect()
    };
    let total_filtered = filtered.len();
    let start = (page.saturating_sub(1)) * page_size;
    let page_items: Vec<&LinkItem> = filtered.into_iter().skip(start).take(page_size).collect();

    let mut page_groups: Vec<TypeGroupWithLinks> = Vec::new();
    if disk_type_filter == "__all__" {
        let mut grouped: HashMap<String, Vec<&LinkItem>> = HashMap::new();
        for item in &page_items {
            grouped.entry(item.disk_type.clone()).or_default().push(*item);
        }
        for dt in &type_order {
            if let Some(links) = grouped.remove(*dt) {
                let owned: Vec<LinkItem> = links.into_iter().map(|l| l.clone()).collect();
                page_groups.push(TypeGroupWithLinks { label: type_friendly(dt), disk_type: dt.to_string(), links: owned });
            }
        }
        for (dt, links) in grouped {
            let owned: Vec<LinkItem> = links.into_iter().map(|l| l.clone()).collect();
            page_groups.push(TypeGroupWithLinks { label: type_friendly(&dt), disk_type: dt, links: owned });
        }
    } else {
        let owned: Vec<LinkItem> = page_items.iter().map(|l| (*l).clone()).collect();
        if !owned.is_empty() {
            page_groups.push(TypeGroupWithLinks {
                label: type_friendly(&disk_type_filter),
                disk_type: disk_type_filter.clone(),
                links: owned,
            });
        }
    }

    let mut ctx = tera::Context::new();
    ctx.insert("keyword", &keyword);
    ctx.insert("source_type", &source_type);
    ctx.insert("disk_type_filter", &disk_type_filter);
    ctx.insert("page", &page);
    ctx.insert("total_links", &total_links);
    ctx.insert("total_filtered", &total_filtered);
    ctx.insert("type_groups", &type_groups);
    ctx.insert("page_groups", &page_groups);
    ctx.insert("domain", &format_domain(&state.config.domain));
    ctx.insert("related_searches", &templates::related_searches(&keyword));

    match templates::render_template(&state.templates, "search.html", ctx) {
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

pub async fn resource_page_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.resource_cache.get(&id) {
        Some(resource) => {
            let mut ctx = tera::Context::new();
            ctx.insert("resource", &resource);
            ctx.insert("domain", &format_domain(&state.config.domain));

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

            match templates::render_template(&state.templates, "resource.html", ctx) {
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
        None => {
            let mut ctx = tera::Context::new();
            ctx.insert("domain", &format_domain(&state.config.domain));
            match templates::render_template(&state.templates, "404.html", ctx) {
                Ok(html) => (
                    StatusCode::NOT_FOUND,
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    html,
                ).into_response(),
                Err(_) => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
            }
        }
    }
}
