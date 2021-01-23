use crate::config::Config;

#[derive(Clone)]
pub struct Debug {
  config: Config,
}

impl Debug {
  pub fn new(config: Config) -> Self {
    Debug { config }
  }

  pub fn log(&self, msg: &str) {
    if self.config.get_env().is_dev() {
      println!("{}", msg);
    }
  }
}
