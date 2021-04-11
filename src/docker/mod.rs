mod container;

use std::{sync::RwLock};

pub use container::{Container, ContainerBuilder};
use container::ContainerInspect;
use reqwest::{Client, Error};

use crate::config::ServerConfig;

lazy_static! {
  static ref URI: RwLock<Option<String>> = RwLock::new(None);
}

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

pub struct DockerContainers {
  uri: String,
  http: Client,
}

impl Docker {
  pub fn client() -> Result<Docker, String> {
    match URI.try_read() {
      Err(_) => Err("Failed to get lock on configured URI".into()),
      Ok(lock) => {
        let uri = lock.as_ref().expect("Docker was not configured");
        Ok(Docker{uri: uri.into(), http: Client::new()})
      }
    }
  }

  pub fn containers(&self) -> DockerContainers {
    DockerContainers{uri: format!("{}/containers", self.uri), http: self.http.clone()}
  }
}


impl DockerContainers {
  pub async fn list(&self) -> Result<Vec<Container>, Error> {
    let res = self.http.get(format!("{}/json", self.uri).as_str()).send().await?.error_for_status();
    match res {
      Ok(resp) => Ok(resp.json::<Vec<Container>>().await?),
      Err(err) => Err(err)
    }
  }

  pub async fn start(&self, id: &str) -> Result<(), Error> {
   self.http.post(format!("{}/{}/start", self.uri, id).as_str()).send().await?.error_for_status().map(|_| ())
  }

  pub async fn stop(&self, id: &str) -> Result<(), Error> {
    self.http.post(format!("{}/{}/stop", self.uri, id).as_str()).send().await?.error_for_status().map(|_| ())?;
    self.http.post(format!("{}/{}/wait", self.uri, id).as_str()).send().await?.error_for_status().map(|_| ())
  }

  pub async fn inspect(&self, id: &str) -> Result<Container, Error> {
    let res = self.http.get(format!("{}/{}/json", self.uri, id).as_str()).send().await?.error_for_status();
    match res {
      Ok(resp) => {
        let ins = resp.json::<ContainerInspect>().await?;
        Ok(ContainerBuilder::default().id(ins.id).name(ins.name).state(ins.state.status).build().unwrap())
      },
      Err(err) => Err(err)
    }
  }
}