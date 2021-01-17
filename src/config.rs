use std::{env, env::VarError};

#[derive(Debug, Clone)]
pub struct Config {
  api_key: String,
  emote_name: String,
  emote_users: Vec<String>,
}

impl Config {
  pub fn new() -> Result<Config, VarError> {
    Ok(Config {
      api_key: env::var("API_KEY")?,
      emote_name: env::var("EMOTE_NAME")?,
      emote_users: env::var("EMOTE_USERS")?
        .split(",")
        .map(|x| x.to_string())
        .collect(),
    })
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
}
