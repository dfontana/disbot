use axum::{
    extract::{Form, Query},
    http::StatusCode,
    response::{Html, Redirect, Response, IntoResponse},
};
use std::collections::HashMap;

use crate::config::{Config, FormData};
use crate::env::Environment;
use crate::web::templates;

pub async fn get_admin(Query(params): Query<HashMap<String, String>>) -> Result<Html<String>, StatusCode> {
    let config = match Config::global_instance().read() {
        Ok(config) => config.clone(),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let success = params.get("success").map(|_| "Configuration saved successfully!");
    let html = templates::render_admin_page(&config, None, success);
    Ok(Html(html))
}

pub async fn post_admin(Form(params): Form<HashMap<String, String>>) -> Response {
    // Parse form data
    let form_data = match parse_form_data(params) {
        Ok(data) => data,
        Err(error) => {
            let config = Config::global_instance().read().map(|c| c.clone()).unwrap_or_default();
            let html = templates::render_admin_page(&config, Some(&error), None);
            return Html(html).into_response();
        }
    };

    // Update configuration
    let config_path = "config.toml"; // TODO: Make this configurable
    let result = {
        let mut config = match Config::global_instance().write() {
            Ok(config) => config,
            Err(_) => {
                let config = Config::global_instance().read().map(|c| c.clone()).unwrap_or_default();
                let html = templates::render_admin_page(&config, Some("Failed to acquire configuration lock"), None);
                return Html(html).into_response();
            }
        };

        // Update config from form data
        match config.update_from_form(&form_data) {
            Ok(_) => {
                // Try to save to file
                match config.to_toml(config_path) {
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
        Err(error) => {
            let config = Config::global_instance().read().map(|c| c.clone()).unwrap_or_default();
            let html = templates::render_admin_page(&config, Some(&error), None);
            Html(html).into_response()
        }
    }
}

fn parse_form_data(params: HashMap<String, String>) -> Result<FormData, String> {
    let emote_name = params.get("emote_name")
        .ok_or("Missing emote_name")?
        .clone();

    let emote_users = params.get("emote_users")
        .unwrap_or(&String::new())
        .clone();

    let env = params.get("env")
        .ok_or("Missing env")?
        .parse::<Environment>()
        .map_err(|e| format!("Invalid environment: {}", e))?;

    let log_level = params.get("log_level")
        .ok_or("Missing log_level")?
        .clone();

    let voice_channel_timeout_seconds = params.get("voice_channel_timeout_seconds")
        .ok_or("Missing voice_channel_timeout_seconds")?
        .parse::<u64>()
        .map_err(|_| "Invalid timeout value")?;

    Ok(FormData {
        emote_name,
        emote_users,
        env,
        log_level,
        voice_channel_timeout_seconds,
    })
}