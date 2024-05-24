// This lint is partially broken, so we'll disable it until it doesn't have
// false positives.
//
// See https://github.com/rust-lang/rust-clippy/pull/12756
#![allow(clippy::assigning_clones)]

use std::sync::Arc;

use axum::http::{header, HeaderValue};
use db::DataAccessProvider;
use state::AppState;
use tokio::{net::TcpListener, signal::unix::SignalKind};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
use tower_layer::Layer;

mod api;
mod auth;
mod conf;
mod db;
mod logging;
mod signal;
mod state;
mod stream;
mod tracker;

/// Creates the service router from the service configuration.
async fn create_router_from_config(
    config: conf::Config,
) -> Result<axum::Router<()>, Box<dyn std::error::Error>> {
    Ok(match &config.database {
        #[cfg(feature = "postgres")]
        conf::Database::Postgres { connection_string } => {
            let data_provider = sqlx::PgPool::connect(connection_string).await?;
            data_provider.migrate().await?;
            println!("Migrations completed successfully.");
            api::create_router(Arc::new(AppState::new(config, data_provider)))
        }
    })
}

/// Middleware function to set `cache-control` headers on static assets.
async fn set_asset_cache_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;

    let cacheable = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map_or(false, |v| v == "application/javascript" || v == "text/css");

    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if cacheable {
            "public, max-age=2592000, immutable"
        } else {
            "no-store"
        }),
    );

    response
}

/// Service entry point.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = conf::load()?;
    let listen = config.http_listen;
    let cors = config.cors_permissive;

    let mut api_router = create_router_from_config(config).await?;
    if cors {
        api_router = api_router.layer(CorsLayer::permissive());
    }

    let router = axum::Router::new()
        .nest("/api", api_router)
        .fallback_service(
            axum::middleware::from_fn(set_asset_cache_headers)
                .layer(ServeDir::new("dist").fallback(ServeFile::new("dist/index.html"))),
        );

    axum::serve(TcpListener::bind(listen).await?, router)
        .with_graceful_shutdown(async {
            match signal::any([SignalKind::interrupt(), SignalKind::terminate()]) {
                Ok(f) => f.await,
                Err(e) => {
                    eprintln!("Unable to listen for shutdown signals: {e}");
                    std::future::pending().await
                }
            }
        })
        .await?;

    Ok(())
}
