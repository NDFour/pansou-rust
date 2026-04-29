mod config;
mod handlers;
mod model;
mod plugin;
mod service;

use std::{sync::Arc, time::Duration};

use axum::{
    response::Redirect,
    routing::{get, post},
    Router,
};
use config::AppConfig;
use service::{CheckService, SearchService};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::ServeDir,
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
        let file_appender = tracing_appender::rolling::weekly(
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

    info!("日志级别: {}", config.log_level);
    info!("配置: {:?}", config);

    let state = Arc::new(AppState {
        config: config.clone(),
        search_service: SearchService::new(config.concurrency, Duration::from_secs(config.cache_ttl), config.max_cache_size),
        check_service: CheckService::new(),
    });

    let static_dir = std::env::current_dir()
        .map(|d| d.join("static"))
        .unwrap_or_else(|_| std::path::PathBuf::from("static"));

    let api_router = Router::new()
        .route("/", get(|| async { Redirect::permanent("/index.html") }))
        .route("/api/search", get(handlers::search_get_handler).post(handlers::search_post_handler))
        .route("/api/check/links", post(handlers::check_handler))
        .route("/api/health", get(handlers::health_handler))
        .nest_service("/static", ServeDir::new(&static_dir))
        .fallback_service(ServeDir::new(&static_dir));

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
