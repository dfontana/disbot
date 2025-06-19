use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::Environment;
use std::sync::RwLock;
use std::{fs, path::Path};

static INSTANCE: Lazy<RwLock<Config>> = Lazy::new(|| RwLock::new(Config::default()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
  pub api_key: String,
  pub app_id: u64,
  pub emote_name: String,
  pub emote_users: Vec<String>,
  #[serde(skip)]
  pub env: Environment,
  pub log_level: String,
  pub voice_channel_timeout_seconds: u64,
  pub db_path: String,
}

impl Default for Config {
  fn default() -> Self {
    Config {
      api_key: "TOKEN".to_string(),
      app_id: 0,
      emote_name: "shrug_dog".to_string(),
      emote_users: vec!["User1".to_string()],
      env: Environment::Dev,
      log_level: "INFO".to_string(),
      voice_channel_timeout_seconds: 600,
      db_path: "disbot.db".to_string(),
    }
  }
}

impl Config {
  pub fn from_toml<P: AsRef<Path>>(
    path: P,
    env: Environment,
  ) -> Result<Config, Box<dyn std::error::Error>> {
    let path_ref = path.as_ref();

    // If file doesn't exist, generate it with defaults
    if !path_ref.exists() {
      info!(
        "Configuration file {} not found. Generating default configuration...",
        path_ref.display()
      );

      // Determine environment from filename
      let env = path_ref
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| name.starts_with("prod"))
        .map(|_| Environment::Prod)
        .unwrap_or(Environment::Dev);

      let default_config = Config {
        env: env.clone(),
        ..Default::default()
      };

      default_config.to_toml(path_ref)?;
      info!(
        "Generated {} - please edit this file with your bot credentials and restart.",
        path_ref.display()
      );
      std::process::exit(0);
    }

    // Load existing file
    let content = fs::read_to_string(path_ref)?;
    let mut config: Config = toml::from_str(&content)?;
    config.env = env;
    Ok(config)
  }

  pub fn to_toml<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
    let toml_string = toml::to_string_pretty(self)?;
    fs::write(path, toml_string)?;
    Ok(())
  }

  pub fn update_from_form(&mut self, form_data: &FormData) -> Result<(), ValidationError> {
    // Validate all fields first
    self.validate_form_data(form_data)?;

    // Update fields
    self.emote_name = form_data.emote_name.clone();
    self.emote_users = form_data
      .emote_users
      .split(',')
      .filter_map(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
          None
        } else {
          Some(trimmed.to_string())
        }
      })
      .collect();

    // Update log level and apply runtime change
    if self.log_level != form_data.log_level {
      self.log_level = form_data.log_level.clone();
      // Apply log level change at runtime
      form_data
        .log_level
        .parse::<tracing::Level>()
        .map_err(|_| ())
        .and_then(|level| crate::set_log_level(level).map_err(|_| ()))
        .unwrap_or_else(|_| warn!("Failed to update runtime log level"));
    }

    self.voice_channel_timeout_seconds = form_data.voice_channel_timeout_seconds;

    Ok(())
  }

  fn validate_form_data(&self, form_data: &FormData) -> Result<(), ValidationError> {
    // Validate emote_name
    if form_data.emote_name.is_empty() {
      return Err(ValidationError::EmoteName(
        "Emote name cannot be empty".to_string(),
      ));
    }
    if !form_data
      .emote_name
      .chars()
      .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
      return Err(ValidationError::EmoteName(
        "Emote name can only contain alphanumeric characters, underscores, and dashes".to_string(),
      ));
    }

    // Validate log_level
    let valid_levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
    if !valid_levels.contains(&form_data.log_level.as_str()) {
      return Err(ValidationError::LogLevel(format!(
        "Log level must be one of: {}",
        valid_levels.join(", ")
      )));
    }

    // Validate timeout
    if form_data.voice_channel_timeout_seconds < 10
      || form_data.voice_channel_timeout_seconds > 3600
    {
      return Err(ValidationError::Timeout(
        "Voice channel timeout must be between 10 and 3600 seconds".to_string(),
      ));
    }

    Ok(())
  }

  pub fn global_instance() -> &'static Lazy<RwLock<Config>> {
    &INSTANCE
  }
}

#[derive(Debug, Clone)]
pub struct FormData {
  pub emote_name: String,
  pub emote_users: String,
  pub log_level: String,
  pub voice_channel_timeout_seconds: u64,
}

#[derive(Debug)]
pub enum ValidationError {
  EmoteName(String),
  LogLevel(String),
  Timeout(String),
}

impl std::fmt::Display for ValidationError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ValidationError::EmoteName(msg) => write!(f, "Invalid emote name: {}", msg),
      ValidationError::LogLevel(msg) => write!(f, "Invalid log level: {}", msg),
      ValidationError::Timeout(msg) => write!(f, "Invalid timeout: {}", msg),
    }
  }
}

impl std::error::Error for ValidationError {}
