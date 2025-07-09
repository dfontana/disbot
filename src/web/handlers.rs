use crate::web::templates;
use crate::{
  config::{Config, FormData},
  persistence::PersistentStore,
};
use axum::{
  extract::{Extension, Form, Query},
  http::{header, StatusCode},
  response::{Html, IntoResponse, Redirect, Response},
};
use humantime::parse_duration;
use std::{collections::HashMap, sync::Arc};

// Helper function to get config or return default
fn get_config_or_default() -> Config {
  Config::global_instance()
    .read()
    .map(|c| c.clone())
    .unwrap_or_default()
}

// Helper function to render error response
fn render_error_response(error: &str, persistence: &Arc<PersistentStore>) -> Html<String> {
  let config = get_config_or_default();
  let checkin_configs = persistence.check_ins().load_all().unwrap_or_default();
  let active_polls = persistence
    .polls()
    .load_all()
    .unwrap_or_default()
    .into_iter()
    .map(|(_, p)| p)
    .collect();
  Html(templates::render_admin_page(
    &config,
    Some(error),
    None,
    checkin_configs,
    active_polls,
  ))
}

pub async fn get_admin(
  Extension(persistence): Extension<Arc<PersistentStore>>,
  Query(params): Query<HashMap<String, String>>,
) -> Result<Html<String>, StatusCode> {
  let config = get_config_or_default();
  let success = params
    .get("success")
    .map(|_| "Configuration saved successfully!");

  let checkin_configs = persistence.check_ins().load_all().unwrap_or_default();
  let active_polls = persistence
    .polls()
    .load_all()
    .unwrap_or_default()
    .into_iter()
    .map(|(_, p)| p)
    .collect();

  Ok(Html(templates::render_admin_page(
    &config,
    None,
    success,
    checkin_configs,
    active_polls,
  )))
}

pub async fn post_admin(
  Extension(config_path): Extension<String>,
  Extension(persistence): Extension<Arc<PersistentStore>>,
  Form(params): Form<HashMap<String, String>>,
) -> Response {
  // Parse form data for regular config update
  let form_data = match parse_form_data(params) {
    Ok(data) => data,
    Err(error) => return render_error_response(&error, &persistence).into_response(),
  };

  // Update configuration
  let result = {
    let mut config = match Config::global_instance().write() {
      Ok(config) => config,
      Err(_) => {
        return render_error_response("Failed to acquire configuration lock", &persistence)
          .into_response()
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
    Err(error) => render_error_response(&error, &persistence).into_response(),
  }
}

fn parse_form_data(params: HashMap<String, String>) -> Result<FormData, String> {
  let emote_name = params
    .get("emote_name")
    .ok_or("Missing emote_name")?
    .clone();

  let emote_users = params.get("emote_users").unwrap_or(&String::new()).clone();

  let log_level = params.get("log_level").ok_or("Missing log_level")?.clone();

  let voice_channel_timeout_str = params
    .get("voice_channel_timeout")
    .ok_or("Missing voice_channel_timeout")?;
  let voice_channel_timeout =
    parse_duration(voice_channel_timeout_str).map_err(|_| "Invalid timeout value")?;

  Ok(FormData {
    emote_name,
    emote_users,
    log_level,
    voice_channel_timeout,
  })
}

pub async fn get_favicon() -> Result<impl IntoResponse, StatusCode> {
  let favicon_data = include_bytes!("../img/shrug-cat.png");

  Ok((
    [(header::CONTENT_TYPE, "image/png")],
    favicon_data.as_slice(),
  ))
}
