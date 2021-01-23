use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub enum Environment {
  PROD,
  DEV,
}

impl Environment {
  pub fn as_file(&self) -> String {
    match &self {
      Environment::PROD => String::from("prod.env"),
      Environment::DEV => String::from("dev.env"),
    }
  }

  pub fn is_dev(&self) -> bool {
    self == &Environment::DEV
  }
}

impl FromStr for Environment {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "prod" => Ok(Environment::PROD),
      "dev" => Ok(Environment::DEV),
      _ => Err("Unknown Environment Given".to_string()),
    }
  }
}

impl Default for Environment {
  fn default() -> Self {
    Environment::PROD
  }
}
