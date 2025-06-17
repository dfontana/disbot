pub mod handlers;
pub mod templates;

use axum::{routing::get, Router};

pub fn create_router() -> Router {
    Router::new()
        .route("/admin", get(handlers::get_admin).post(handlers::post_admin))
}

pub async fn start_server(_config_path: String, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_router();

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Admin web server running on http://0.0.0.0:{}/admin", port);
    
    axum::serve(listener, app).await?;
    Ok(())
}