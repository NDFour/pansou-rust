use axum::{
    http::{header, StatusCode, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

use crate::constants::cache;
use crate::constants::cache_ext;

#[derive(RustEmbed)]
#[folder = "static/"]
pub struct Assets;

pub async fn serve_embedded(uri: Uri) -> impl IntoResponse {
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
        None => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>页面未找到 - 盘盘侠</title>
<style>
:root{--color-brand:#e85d3a;--color-stone:#6b6b6b;--color-ivory:#faf8f5;--color-border-warm:#e8e0d5;--radius-sm:8px}
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;background:#faf8f5;color:#2c2c2c;min-height:100vh;display:flex;align-items:center;justify-content:center}
.container{text-align:center;padding:2rem;max-width:520px}
h1{font-size:1.5rem;font-weight:700;margin:1rem 0 .5rem}
p{color:#6b6b6b;margin-bottom:1.5rem}
a{color:#e85d3a;text-decoration:none}
.search-bar{display:flex;max-width:400px;margin:0 auto 1.5rem}
.search-bar-input{flex:1;padding:.75rem 1rem;border:2px solid #e8e0d5;border-radius:8px 0 0 8px;font-size:1rem;outline:none}
.search-bar-input:focus{border-color:#e85d3a}
.search-bar-btn{padding:.75rem 1.25rem;background:#e85d3a;color:#fff;border:none;border-radius:0 8px 8px 0;cursor:pointer;font-size:1rem;white-space:nowrap}
</style>
</head>
<body>
<div class="container">
<div style="font-size:3rem">🔍</div>
<h1>页面未找到</h1>
<p>抱歉，您访问的页面不存在。试试搜索您想要的资源吧。</p>
<div class="search-bar">
<input type="text" id="s" class="search-bar-input" placeholder="输入关键词搜索资源..." autofocus onkeydown="if(event.key==='Enter')search()">
<button class="search-bar-btn" onclick="search()">搜索</button>
</div>
<a href="/">← 返回首页</a>
</div>
<script>function search(){var q=document.getElementById('s').value.trim();if(q)location.href='/search?kw='+encodeURIComponent(q)}</script>
</body>
</html>"#,
        ).into_response(),
    }
}
