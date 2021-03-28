use crate::Environment;
use lazy_static::lazy_static;
use std::sync::RwLock;
use std::{env, env::VarError};

lazy_static! {
  static ref INSTANCE: RwLock<Config> = RwLock::new(Config::default());
}

#[derive(Debug, Clone)]
pub struct Config {
  api_key: String,
  emote_name: String,
  emote_users: Vec<String>,
  env: Environment,
  server_mac: String,
}

impl Default for Config {
  fn default() -> Self {
    Config {
      api_key: "".to_owned(),
      emote_name: "".to_owned(),
      emote_users: Vec::new(),
      env: Environment::DEV,
      server_mac: "".to_owned(),
    }
  }
}

impl Config {
  pub fn set(env: Environment) -> Result<Config, VarError> {
    let c = Config {
      api_key: env::var("API_KEY")?,
      emote_name: env::var("EMOTE_NAME")?,
      emote_users: env::var("EMOTE_USERS")?
        .split(",")
        .map(|x| x.to_string())
        .collect(),
      env,
      server_mac: env::var("SERVER_MAC")?,
    };
    if let Ok(mut inst) = INSTANCE.try_write() {
      *inst = c.clone();
    }
    Ok(c.clone())
  }

  pub fn inst() -> Result<Config, String> {
    Ok(
      INSTANCE
        .try_read()
        .map_err(|_| "Failed to get config read lock")?
        .clone(),
    )
  }

  pub fn get_api_key(&self) -> &String {
    &self.api_key
  }

  pub fn get_emote_name(&self) -> &String {
    &self.emote_name
  }

  pub fn get_emote_users(&self) -> &Vec<String> {
    &self.emote_users
  }

  pub fn get_env(&self) -> &Environment {
    &self.env
  }

  pub fn get_server_mac(&self) -> &String {
    &self.server_mac
  }
}
