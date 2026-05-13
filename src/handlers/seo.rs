use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use chrono::TimeZone;

use crate::{
    constants::HOT_KEYWORDS,
    AppState,
};

use super::urlencoding;

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
         Crawl-delay: 1\n\
         Sitemap: {}/sitemap.xml\n",
        domain
    );
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}

pub async fn sitemap_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let domain = &state.config.domain;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let resource_ids = state.resource_cache.all_ids();
    let per_sitemap = 500;

    let sitemap_type = params.get("type").cloned().unwrap_or_default();

    if sitemap_type == "main" {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);

        xml.push_str(&format!(
            "<url><loc>{}/</loc><lastmod>{}</lastmod><priority>1.0</priority><changefreq>daily</changefreq></url>",
            domain, today
        ));

        let hot_keywords = HOT_KEYWORDS;
        for kw in hot_keywords {
            let encoded = urlencoding(kw);
            xml.push_str(&format!(
                "<url><loc>{}/search?kw={}</loc><lastmod>{}</lastmod><priority>0.8</priority><changefreq>daily</changefreq></url>",
                domain, encoded, today
            ));
        }

        xml.push_str("</urlset>");
        return (StatusCode::OK, [(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml);
    }

    if sitemap_type == "resources" {
        let page: usize = params.get("page").and_then(|v| v.parse().ok()).unwrap_or(0);
        let start = page * per_sitemap;
        let end = (start + per_sitemap).min(resource_ids.len());

        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);

        for id in &resource_ids[start..end] {
            let lastmod = state.resource_cache.get(id)
                .map(|r| {
                    chrono::Utc.timestamp_opt(r.created_at, 0)
                        .single()
                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| today.clone())
                })
                .unwrap_or_else(|| today.clone());
            xml.push_str(&format!(
                "<url><loc>{}/resource/{}</loc><lastmod>{}</lastmod><priority>0.7</priority><changefreq>weekly</changefreq></url>",
                domain, id, lastmod
            ));
        }

        xml.push_str("</urlset>");
        return (StatusCode::OK, [(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml);
    }

    // 无参数：返回 sitemap index 或完整 sitemap
    if resource_ids.len() > per_sitemap {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push_str(r#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);

        xml.push_str(&format!(
            "<sitemap><loc>{}/sitemap.xml?type=main</loc><lastmod>{}</lastmod></sitemap>",
            domain, today
        ));

        let num_sitemaps = (resource_ids.len() + per_sitemap - 1) / per_sitemap;
        for i in 0..num_sitemaps {
            xml.push_str(&format!(
                "<sitemap><loc>{}/sitemap.xml?type=resources&amp;page={}</loc><lastmod>{}</lastmod></sitemap>",
                domain, i, today
            ));
        }

        xml.push_str("</sitemapindex>");
        return (StatusCode::OK, [(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml);
    }

    // 资源少时生成完整 sitemap
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);

    xml.push_str(&format!(
        "<url><loc>{}/</loc><lastmod>{}</lastmod><priority>1.0</priority><changefreq>daily</changefreq></url>",
        domain, today
    ));

    let hot_keywords = [
        "流浪地球", "庆余年", "凡人修仙传", "三体", "哪吒", "封神",
        "鬼灭之刃", "海贼王", "火影忍者", "原神",
    ];
    for kw in hot_keywords {
        let encoded = urlencoding(kw);
        xml.push_str(&format!(
            "<url><loc>{}/search?kw={}</loc><lastmod>{}</lastmod><priority>0.8</priority><changefreq>daily</changefreq></url>",
            domain, encoded, today
        ));
    }

    for id in &resource_ids {
        let lastmod = state.resource_cache.get(id)
            .map(|r| {
                chrono::Utc.timestamp_opt(r.created_at, 0)
                    .single()
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| today.clone())
            })
            .unwrap_or_else(|| today.clone());
        xml.push_str(&format!(
            "<url><loc>{}/resource/{}</loc><lastmod>{}</lastmod><priority>0.7</priority><changefreq>weekly</changefreq></url>",
            domain, id, lastmod
        ));
    }

    xml.push_str("</urlset>");
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml)
}
