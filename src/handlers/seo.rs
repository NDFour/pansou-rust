use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
};

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
         Disallow: /api/\n\
         Crawl-delay: 1\n\
         Sitemap: {}/sitemap.xml\n",
        domain
    );
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}

pub async fn sitemap_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let domain = &state.config.domain;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);

    xml.push_str(&format!(
        "<url><loc>{}/</loc><lastmod>{}</lastmod><priority>1.0</priority><changefreq>daily</changefreq></url>",
        domain, today
    ));

    let hot_keywords = state.resource_cache.hot_keywords(20);
    let keywords: Vec<&str> = if hot_keywords.is_empty() {
        HOT_KEYWORDS.to_vec()
    } else {
        hot_keywords.iter().map(|s| s.as_str()).collect()
    };
    for kw in keywords {
        let encoded = urlencoding(kw);
        xml.push_str(&format!(
            "<url><loc>{}/search?kw={}</loc><lastmod>{}</lastmod><priority>0.8</priority><changefreq>daily</changefreq></url>",
            domain, encoded, today
        ));
    }

    xml.push_str("</urlset>");
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml)
}
