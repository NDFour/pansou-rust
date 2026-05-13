mod config;
mod constants;
mod handlers;
mod model;
mod plugin;
mod post_search;
mod assets;
mod resource_cache;
mod service;
mod templates;

use std::{sync::Arc, time::Duration};

use axum::{
    routing::{get, post},
    Router,
};
use config::AppConfig;
use crate::resource_cache::ResourceCache;
use service::{CheckService, SearchService};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, info_span};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    config: AppConfig,
    search_service: SearchService,
    check_service: CheckService,
    templates: Arc<tera::Tera>,
    resource_cache: ResourceCache,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_file();

    let env_filter = tracing_subscriber::EnvFilter::new(&config.log_level);
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .compact();

    if config.log_file.is_empty() {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .init();
    } else {
        let file_appender = tracing_appender::rolling::daily(
            std::path::Path::new(&config.log_file).parent().unwrap_or(std::path::Path::new(".")),
            std::path::Path::new(&config.log_file).file_name().unwrap_or(std::ffi::OsStr::new("app.log")),
        );
        let file_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_target(false)
            .with_writer(file_appender);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .with(file_layer)
            .init();
    }

    info!("版本号 v{}", env!("CARGO_PKG_VERSION"));
    info!("配置: {:?}", config);

    let templates = Arc::new(
        templates::init_templates(&config.templates_dir)
            .expect("无法加载 HTML 模板")
    );

    let state = Arc::new(AppState {
        config: config.clone(),
        search_service: SearchService::new(config.concurrency, Duration::from_secs(config.cache_ttl), config.max_cache_size, &config.post_search_endpoint),
        check_service: CheckService::new(),
        templates,
        resource_cache: ResourceCache::new(config.max_cache_size),
    });

    let api_router = Router::new()
        .route("/", get(assets::serve_embedded))
        .route("/api/search", get(handlers::search_get_handler).post(handlers::search_post_handler))
        .route("/api/check/links", post(handlers::check_handler))
        .route("/api/stats/metric", post(handlers::metric_handler))
        .route("/api/health", get(handlers::health_handler))
        .route("/api/hot-keywords", get(handlers::hot_keywords_handler))
        .route("/robots.txt", get(handlers::robots_handler))
        .route("/sitemap.xml", get(handlers::sitemap_handler))
        .route("/search", get(handlers::search_page_handler))
        .fallback(assets::serve_embedded);

    let app = api_router
        .layer(CompressionLayer::new())
        .layer(
            TraceLayer::new_for_http().make_span_with(|_request: &axum::http::Request<_>| {
                let request_id = Uuid::new_v4().simple().to_string();
                info_span!("request", %request_id)
            }),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Rust服务启动: http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
