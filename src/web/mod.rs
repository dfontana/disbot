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
  port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
  let app = create_router(config_path, persistence);

  let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
  println!("Admin web server running on http://0.0.0.0:{}/admin", port);

  axum::serve(listener, app).await?;
  Ok(())
}
