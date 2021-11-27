mod container;

use std::sync::RwLock;

use container::DockerContainers;
pub use container::{Container, ContainerBuilder};
use once_cell::sync::Lazy;
use reqwest::Client;

use crate::config::ServerConfig;

static URI: Lazy<RwLock<Option<String>>> = Lazy::new(|| RwLock::new(None));

pub fn configure(cfg: &ServerConfig) -> Result<(), String> {
  let mut inst = URI
    .try_write()
    .map_err(|_| "Failed to get lock on docker client setup")?;
  *inst = Some(format!("http://{}:2375", cfg.ip));
  Ok(())
}

pub struct Docker {
  uri: String,
  http: Client,
}

impl Docker {
  pub fn client() -> Result<Docker, String> {
    match URI.try_read() {
      Err(_) => Err("Failed to get lock on configured URI".into()),
      Ok(lock) => {
        let uri = lock.as_ref().expect("Docker was not configured");
        Ok(Docker {
          uri: uri.into(),
          http: Client::new(),
        })
      }
    }
  }

  pub fn containers(&self) -> DockerContainers {
    DockerContainers::new(&format!("{}/containers", self.uri), &self.http)
  }
}
