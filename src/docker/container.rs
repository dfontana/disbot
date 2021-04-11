use std::collections::HashMap;

use reqwest::{Client, Error};
use serde::Deserialize;

#[derive(Clone, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ContainerState {
  CREATED,
  RESTARTING,
  RUNNING,
  REMOVING,
  PAUSED,
  EXITED,
  DEAD,
}

#[derive(Builder, Default, Deserialize, Debug, PartialEq)]
pub struct Container {
  #[serde(rename = "Id")]
  pub id: String,
  #[serde(rename = "Labels")]
  #[builder(setter(each = "label"))]
  pub labels: HashMap<String, String>,
  #[serde(rename = "State")]
  pub state: ContainerState,
}

#[derive(Builder, Deserialize, Debug, PartialEq)]
pub struct ContainerInspect {
  #[serde(rename = "Id")]
  pub id: String,
  #[serde(rename = "Config")]
  pub config: ContainerInspectConfig,
  #[serde(rename = "State")]
  pub state: ContainerInspectState,
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct ContainerInspectState {
  #[serde(rename = "Status")]
  pub status: ContainerState,
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct ContainerInspectConfig {
  #[serde(rename = "Labels")]
  pub labels: HashMap<String, String>,
}

impl Default for ContainerState {
  fn default() -> Self {
    ContainerState::EXITED
  }
}

pub struct DockerContainers {
  uri: String,
  http: Client,
}

impl DockerContainers {
  pub fn new(uri: &str, http: &Client) -> Self {
    DockerContainers {
      uri: uri.into(),
      http: http.clone(),
    }
  }

  pub async fn list(&self) -> Result<Vec<Container>, Error> {
    let res = self
      .http
      .get(format!("{}/json?all=true", self.uri).as_str())
      .send()
      .await?
      .error_for_status();
    match res {
      Ok(resp) => Ok(resp.json::<Vec<Container>>().await?),
      Err(err) => Err(err),
    }
  }

  pub async fn start(&self, id: &str) -> Result<(), Error> {
    self
      .http
      .post(format!("{}/{}/start", self.uri, id).as_str())
      .send()
      .await?
      .error_for_status()
      .map(|_| ())
  }

  pub async fn stop(&self, id: &str) -> Result<(), Error> {
    self
      .http
      .post(format!("{}/{}/stop", self.uri, id).as_str())
      .send()
      .await?
      .error_for_status()
      .map(|_| ())?;
    self
      .http
      .post(format!("{}/{}/wait", self.uri, id).as_str())
      .send()
      .await?
      .error_for_status()
      .map(|_| ())
  }

  pub async fn inspect(&self, id: &str) -> Result<Container, Error> {
    let res = self
      .http
      .get(format!("{}/{}/json", self.uri, id).as_str())
      .send()
      .await?
      .error_for_status();
    match res {
      Ok(resp) => {
        let ins = resp.json::<ContainerInspect>().await?;
        Ok(
          ContainerBuilder::default()
            .id(ins.id)
            .labels(ins.config.labels)
            .state(ins.state.status)
            .build()
            .unwrap(),
        )
      }
      Err(err) => Err(err),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_deserialize_containerlist() {
    let input = "[{\"Id\":\"4ad194bf2a9e177d48f11441f21cf9b97098de973434f8db40470e8e7a3551df\",\"Labels\":{\"game\":\"minecraft\",\"version\":\"valhelsia\"},\"Command\":\"/start\",\"State\":\"running\"}]";
    let actual: Vec<Container> = serde_json::from_str(input).unwrap();
    let expected = vec![ContainerBuilder::default()
      .id("4ad194bf2a9e177d48f11441f21cf9b97098de973434f8db40470e8e7a3551df".into())
      .label(("game".into(), "minecraft".into()))
      .label(("version".into(), "valhelsia".into()))
      .state(ContainerState::RUNNING)
      .build()
      .unwrap()];
    assert_eq!(actual, expected);
  }

  #[test]
  fn test_deserialize_containerinspect() {
    let input = "{\"Id\":\"4ad194bf2a9e177d48f11441f21cf9b97098de973434f8db40470e8e7a3551df\",\"Config\":{\"Labels\":{\"game\":\"minecraft\",\"version\":\"valhelsia\"}},\"State\":{\"Status\":\"running\"}}";
    let actual: ContainerInspect = serde_json::from_str(input).unwrap();
    let mut labels: HashMap<String, String> = HashMap::new();
    labels.insert("game".into(), "minecraft".into());
    labels.insert("version".into(), "valhelsia".into());
    let expected = ContainerInspectBuilder::default()
      .id("4ad194bf2a9e177d48f11441f21cf9b97098de973434f8db40470e8e7a3551df".into())
      .config(ContainerInspectConfig { labels })
      .state(ContainerInspectState {
        status: ContainerState::RUNNING,
      })
      .build()
      .unwrap();
    assert_eq!(actual, expected);
  }
}
