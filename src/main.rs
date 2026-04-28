mod auth;
mod config;
mod handlers;
mod model;
mod plugin;
mod service;

use std::sync::Arc;

use axum::{
    middleware,
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
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    config: AppConfig,
    search_service: SearchService,
    check_service: CheckService,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .compact()
        .init();

    info!("Rust服务启动");

    let config = AppConfig::from_env();
    let state = Arc::new(AppState {
        config: config.clone(),
        search_service: SearchService::new(config.go_compat_url.clone()),
        check_service: CheckService::new(config.go_compat_url.clone()),
    });

    let static_dir = std::env::current_dir()
        .map(|d| d.join("static"))
        .unwrap_or_else(|_| std::path::PathBuf::from("static"));

    let api_router = Router::new()
        .route("/", get(|| async { Redirect::permanent("/index.html") }))
        .route("/api/auth/login", post(handlers::login_handler))
        .route("/api/auth/verify", post(handlers::verify_handler))
        .route("/api/auth/logout", post(handlers::logout_handler))
        .route("/api/search", get(handlers::search_get_handler).post(handlers::search_post_handler))
        .route("/api/check/links", post(handlers::check_handler))
        .route("/api/health", get(handlers::health_handler))
        .nest_service("/static", ServeDir::new(&static_dir))
        .fallback_service(ServeDir::new(&static_dir));

    let app = api_router
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Rust服务启动: http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
