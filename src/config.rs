use once_cell::sync::Lazy;

use crate::Environment;
use std::sync::RwLock;
use std::{env, env::VarError};

static INSTANCE: Lazy<RwLock<Config>> = Lazy::new(|| RwLock::new(Config::default()));

#[derive(Debug, Clone)]
pub struct Config {
  pub api_key: String,
  pub emote_name: String,
  pub emote_users: Vec<String>,
  pub env: Environment,
  pub log_level: String,
  pub server: ServerConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
  pub mac: String,
  pub ip: String,
  pub user: String,
  pub port: usize,
}

impl Default for ServerConfig {
  fn default() -> Self {
    ServerConfig {
      mac: "".to_owned(),
      ip: "".to_owned(),
      user: "".to_owned(),
      port: 0,
    }
  }
}

impl Default for Config {
  fn default() -> Self {
    Config {
      api_key: "".to_owned(),
      emote_name: "".to_owned(),
      emote_users: Vec::new(),
      env: Environment::Dev,
      log_level: "INFO".to_string(),
      server: ServerConfig::default(),
    }
  }
}

impl Config {
  pub fn set(env: Environment) -> Result<Config, VarError> {
    let c = Config {
      api_key: env::var("API_KEY")?,
      emote_name: env::var("EMOTE_NAME")?,
      emote_users: env::var("EMOTE_USERS")?
        .split(',')
        .map(|x| x.to_string())
        .collect(),
      env,
      log_level: env::var("LOG_LEVEL")?,
      server: ServerConfig {
        mac: env::var("SERVER_MAC")?,
        ip: env::var("SERVER_IP")?,
        user: env::var("SERVER_USER")?,
        port: env::var("SERVER_DOCKER_PORT")
          .map(|v| {
            v.parse::<usize>()
              .expect("SERVER_DOCKER_PORT not a valid number")
          })
          .unwrap_or(2375),
      },
    };
    if let Ok(mut inst) = INSTANCE.try_write() {
      *inst = c.clone();
    }
    Ok(c)
  }
}
