use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::Environment;
use std::sync::RwLock;
use std::{env, env::VarError, fs, path::Path};

static INSTANCE: Lazy<RwLock<Config>> = Lazy::new(|| RwLock::new(Config::default()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
  pub api_key: String,
  pub app_id: u64,
  pub emote_name: String,
  pub emote_users: Vec<String>,
  pub env: Environment,
  pub log_level: String,
  pub voice_channel_timeout_seconds: u64,
}

impl Default for Config {
  fn default() -> Self {
    Config {
      api_key: "".to_owned(),
      app_id: 0,
      emote_name: "".to_owned(),
      emote_users: Vec::new(),
      env: Environment::Dev,
      log_level: "INFO".to_string(),
      voice_channel_timeout_seconds: 10,
    }
  }
}

impl Config {
  pub fn set(env: Environment) -> Result<Config, VarError> {
    let c = Config {
      api_key: env::var("API_KEY")?,
      app_id: env::var("APP_ID")?
        .parse::<u64>()
        .map_err(|_| VarError::NotPresent)?,
      emote_name: env::var("EMOTE_NAME")?,
      emote_users: env::var("EMOTE_USERS")?
        .split(',')
        .map(|x| x.to_string())
        .collect(),
      env,
      log_level: env::var("LOG_LEVEL")?,
      voice_channel_timeout_seconds: env::var("TIMEOUT")?
        .parse::<u64>()
        .map_err(|_| VarError::NotPresent)?,
    };
    if let Ok(mut inst) = INSTANCE.try_write() {
      *inst = c.clone();
    }
    Ok(c)
  }

  pub fn from_toml<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
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
    self.emote_users = form_data.emote_users
      .split(',')
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
      .collect();
    self.env = form_data.env.clone();
    self.log_level = form_data.log_level.clone();
    self.voice_channel_timeout_seconds = form_data.voice_channel_timeout_seconds;
    
    Ok(())
  }

  fn validate_form_data(&self, form_data: &FormData) -> Result<(), ValidationError> {
    // Validate emote_name
    if form_data.emote_name.is_empty() {
      return Err(ValidationError::InvalidEmoteName("Emote name cannot be empty".to_string()));
    }
    if !form_data.emote_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
      return Err(ValidationError::InvalidEmoteName("Emote name can only contain alphanumeric characters, underscores, and dashes".to_string()));
    }

    // Validate log_level
    let valid_levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
    if !valid_levels.contains(&form_data.log_level.as_str()) {
      return Err(ValidationError::InvalidLogLevel(format!("Log level must be one of: {}", valid_levels.join(", "))));
    }

    // Validate timeout
    if form_data.voice_channel_timeout_seconds < 10 || form_data.voice_channel_timeout_seconds > 3600 {
      return Err(ValidationError::InvalidTimeout("Voice channel timeout must be between 10 and 3600 seconds".to_string()));
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
  pub env: Environment,
  pub log_level: String,
  pub voice_channel_timeout_seconds: u64,
}

#[derive(Debug)]
pub enum ValidationError {
  InvalidEmoteName(String),
  InvalidLogLevel(String),
  InvalidTimeout(String),
}

impl std::fmt::Display for ValidationError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ValidationError::InvalidEmoteName(msg) => write!(f, "Invalid emote name: {}", msg),
      ValidationError::InvalidLogLevel(msg) => write!(f, "Invalid log level: {}", msg),
      ValidationError::InvalidTimeout(msg) => write!(f, "Invalid timeout: {}", msg),
    }
  }
}

impl std::error::Error for ValidationError {}
