pub mod handlers;
pub mod templates;

use axum::{routing::get, Extension, Router};

pub fn create_router(config_path: String) -> Router {
  Router::new()
    .route(
      "/admin",
      get(handlers::get_admin).post(handlers::post_admin),
    )
    .layer(Extension(config_path))
}

pub async fn start_server(
  config_path: String,
  port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
  let app = create_router(config_path);

  let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
  println!("Admin web server running on http://0.0.0.0:{}/admin", port);

  axum::serve(listener, app).await?;
  Ok(())
}
