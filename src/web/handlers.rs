use axum::{
  extract::{Extension, Form, Query},
  http::{header, StatusCode},
  response::{Html, IntoResponse, Redirect, Response},
};
use std::collections::HashMap;

use crate::config::{Config, FormData};
use crate::web::templates;

// Helper function to get config or return default
fn get_config_or_default() -> Config {
  Config::global_instance()
    .read()
    .map(|c| c.clone())
    .unwrap_or_default()
}

// Helper function to render error response
fn render_error_response(error: &str) -> Html<String> {
  let config = get_config_or_default();
  Html(templates::render_admin_page(&config, Some(error), None))
}

pub async fn get_admin(
  Query(params): Query<HashMap<String, String>>,
) -> Result<Html<String>, StatusCode> {
  let config = get_config_or_default();
  let success = params
    .get("success")
    .map(|_| "Configuration saved successfully!");
  Ok(Html(templates::render_admin_page(&config, None, success)))
}

pub async fn post_admin(
  Extension(config_path): Extension<String>,
  Form(params): Form<HashMap<String, String>>,
) -> Response {
  // Parse form data
  let form_data = match parse_form_data(params) {
    Ok(data) => data,
    Err(error) => return render_error_response(&error).into_response(),
  };

  // Update configuration
  let result = {
    let mut config = match Config::global_instance().write() {
      Ok(config) => config,
      Err(_) => {
        return render_error_response("Failed to acquire configuration lock").into_response()
      }
    };

    // Update config from form data
    match config.update_from_form(&form_data) {
      Ok(_) => {
        // Try to save to file
        match config.to_toml(&config_path) {
          Ok(_) => Ok(()),
          Err(e) => {
            // Rollback not needed since we haven't persisted the changes
            Err(format!("Failed to save configuration: {}", e))
          }
        }
      }
      Err(e) => Err(e.to_string()),
    }
  };

  match result {
    Ok(_) => {
      // Redirect to show success
      Redirect::to("/admin?success=1").into_response()
    }
    Err(error) => render_error_response(&error).into_response(),
  }
}

fn parse_form_data(params: HashMap<String, String>) -> Result<FormData, String> {
  let emote_name = params
    .get("emote_name")
    .ok_or("Missing emote_name")?
    .clone();

  let emote_users = params.get("emote_users").unwrap_or(&String::new()).clone();

  let log_level = params.get("log_level").ok_or("Missing log_level")?.clone();

  let voice_channel_timeout_seconds = params
    .get("voice_channel_timeout_seconds")
    .ok_or("Missing voice_channel_timeout_seconds")?
    .parse::<u64>()
    .map_err(|_| "Invalid timeout value")?;

  Ok(FormData {
    emote_name,
    emote_users,
    log_level,
    voice_channel_timeout_seconds,
  })
}

pub async fn get_favicon() -> Result<impl IntoResponse, StatusCode> {
  let favicon_data = include_bytes!("../img/shrug-dog.png");

  Ok((
    [(header::CONTENT_TYPE, "image/png")],
    favicon_data.as_slice(),
  ))
}
