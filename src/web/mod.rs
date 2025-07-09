pub mod handlers;
pub mod templates;

use crate::{persistence::PersistentStore, WebBindAddress};
use axum::{routing::get, Extension, Router};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub fn create_router(config_path: String, persistence: Arc<PersistentStore>) -> Router {
  Router::new()
    .route(
      "/admin",
      get(handlers::get_admin).post(handlers::post_admin),
    )
    .route("/favicon.ico", get(handlers::get_favicon))
    .layer(Extension(config_path))
    .layer(Extension(persistence))
}

pub async fn start_server(
  config_path: String,
  persistence: Arc<PersistentStore>,
  bind_address: WebBindAddress,
  port: u16,
  token: CancellationToken,
) -> Result<(), anyhow::Error> {
  let app = create_router(config_path, persistence);

  // Resolve bind address
  let resolved_address = match bind_address {
    WebBindAddress::Lan => local_ip_address::local_ip()?.to_string(),
    WebBindAddress::Ip(ip) => ip,
  };

  let listener = tokio::net::TcpListener::bind(format!("{}:{}", resolved_address, port)).await?;
  println!(
    "Admin web server running on http://{}:{}/admin",
    resolved_address, port
  );

  axum::serve(listener, app)
    .with_graceful_shutdown(async move {
      token.cancelled().await;
      info!("Web server shutdown signal received");
    })
    .await?;
  Ok(())
}
