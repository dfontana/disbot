pub mod handlers;
pub mod templates;

use crate::persistence::PersistentStore;
use axum::{routing::get, Extension, Router};
use std::sync::Arc;

pub fn create_router(config_path: String, persistence: Arc<PersistentStore>) -> Router {
  Router::new()
    .route(
      "/admin",
      get(handlers::get_admin).post(handlers::post_admin),
    )
    .route(
      "/admin/checkins",
      get(handlers::get_checkin_admin).post(handlers::post_checkin_admin),
    )
    .route("/favicon.ico", get(handlers::get_favicon))
    .layer(Extension(config_path))
    .layer(Extension(persistence))
}

pub async fn start_server(
  config_path: String,
  persistence: Arc<PersistentStore>,
  bind_address: String,
  port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
  let app = create_router(config_path, persistence);

  // Resolve bind address
  let resolved_address = if bind_address == "lan" {
    match local_ip_address::local_ip() {
      Ok(ip) => ip.to_string(),
      Err(e) => {
        eprintln!(
          "Failed to detect LAN IP address: {}. Falling back to 127.0.0.1",
          e
        );
        "127.0.0.1".to_string()
      }
    }
  } else {
    bind_address
  };

  let listener = tokio::net::TcpListener::bind(format!("{}:{}", resolved_address, port)).await?;
  println!(
    "Admin web server running on http://{}:{}/admin",
    resolved_address, port
  );

  axum::serve(listener, app).await?;
  Ok(())
}
