use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
  #[default]
  Prod,
  Dev,
}

impl Environment {
  pub fn as_toml_file(&self) -> String {
    match &self {
      Environment::Prod => String::from("prod.toml"),
      Environment::Dev => String::from("dev.toml"),
    }
  }
}

impl FromStr for Environment {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "prod" => Ok(Environment::Prod),
      "dev" => Ok(Environment::Dev),
      _ => Err("Unknown Environment Given".to_string()),
    }
  }
}
