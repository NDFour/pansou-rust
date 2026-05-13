use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, StatusCode, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

use crate::constants::{cache, cache_ext, templates};
use crate::handlers::format_domain;
use crate::templates::render_template;
use crate::AppState;

#[derive(RustEmbed)]
#[folder = "static/"]
pub struct Assets;

pub async fn serve_embedded(uri: Uri, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = path.strip_prefix("static/").unwrap_or(path);
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let content_type = axum::http::HeaderValue::from_str(mime.as_ref())
                .unwrap_or(axum::http::HeaderValue::from_static("application/octet-stream"));

            let ext = path.rsplit('.').next().unwrap_or("");
            let cache_control = if cache_ext::LONG.contains(&ext) {
                cache::CSS_JS
            } else if cache_ext::VERY_LONG.contains(&ext) {
                cache::IMG_FONT
            } else {
                cache::DEFAULT
            };

            (
                [
                    (header::CONTENT_TYPE, content_type),
                    (header::CACHE_CONTROL, axum::http::HeaderValue::from_static(cache_control)),
                ],
                content.data.into_owned(),
            ).into_response()
        }
        None => {
            let mut ctx = tera::Context::new();
            ctx.insert("domain", &format_domain(&state.config.domain));
            match render_template(&state.templates, templates::NOT_FOUND, ctx) {
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
