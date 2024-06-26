use once_cell::sync::Lazy;

use crate::Environment;
use std::sync::RwLock;
use std::{env, env::VarError};

static INSTANCE: Lazy<RwLock<Config>> = Lazy::new(|| RwLock::new(Config::default()));

#[derive(Debug, Clone)]
pub struct Config {
  pub api_key: String,
  pub app_id: u64,
  pub emote_name: String,
  pub emote_users: Vec<String>,
  pub env: Environment,
  pub log_level: String,
  pub timeout: u64, // Seconds
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
      timeout: 10,
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
      timeout: env::var("TIMEOUT")?
        .parse::<u64>()
        .map_err(|_| VarError::NotPresent)?,
    };
    if let Ok(mut inst) = INSTANCE.try_write() {
      *inst = c.clone();
    }
    Ok(c)
  }
}
