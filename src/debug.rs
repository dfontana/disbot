use crate::config::Config;
use crate::env::Environment;
use lazy_static::lazy_static;
use std::sync::RwLock;

lazy_static! {
  static ref INSTANCE: RwLock<Option<Environment>> = RwLock::new(None);
}

pub struct Debug {
  location: String,
}

pub fn configure(config: &Config) -> Result<(), String> {
  let mut inst = INSTANCE
    .try_write()
    .map_err(|_| "Failed to get lock on debug instance")?;
  *inst = Some(config.get_env().clone());
  Ok(())
}

impl Debug {
  pub fn inst(location: &str) -> Self {
    Debug {
      location: location.to_owned(),
    }
  }

  pub fn log(&self, msg: &str) {
    match INSTANCE.try_read() {
      Err(_) => println!("Failed to aquire debug read lock"),
      Ok(lock) => match *lock {
        Some(Environment::DEV) => println!("[{}] {}", self.location, msg),
        None => println!("Debug ran without configuration"),
        _ => (),
      },
    }
  }
}
