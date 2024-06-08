use anyhow::anyhow;
use bollard::{
  container::{ListContainersOptions, StopContainerOptions},
  service::{ContainerStateStatusEnum, ContainerSummary},
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Docker {
  client: bollard::Docker,
}

impl Docker {
  pub fn new() -> Result<Docker, anyhow::Error> {
    Ok(Docker {
      client: bollard::Docker::connect_with_socket_defaults()?,
    })
  }

  pub async fn list(&self) -> Result<Vec<ContainerSummary>, anyhow::Error> {
    let list_container_filters: HashMap<String, Vec<String>> = HashMap::new();
    // TODO: Use a label filter and set that label ("shibba:true") on each container that can
    //       be managed. Should only work on containers that have been started once, eg container is
    //       already made. No need to interact with compose.
    //       You'll also want labels for (Game, Version) to help inform what to start/stop
    // list_container_filters.insert("status", vec!["running"]);

    self
      .client
      .list_containers(Some(ListContainersOptions {
        all: true,
        filters: list_container_filters,
        ..Default::default()
      }))
      .await
      .map_err(|e| anyhow::anyhow!(e))
  }

  pub async fn status(&self, name: &str) -> Result<ContainerStateStatusEnum, anyhow::Error> {
    self
      .client
      .inspect_container(name, None)
      .await
      .map_err(|e| anyhow::anyhow!(e))
      .and_then(|res| {
        res
          .state
          .and_then(|s| s.status)
          .ok_or(anyhow!("Container in Unknown State"))
      })
  }

  pub async fn start(&self, name: &str) -> Result<(), anyhow::Error> {
    self
      .client
      .start_container::<String>(name, None)
      .await
      .map_err(|e| anyhow::anyhow!(e))
  }

  pub async fn stop(&self, name: &str) -> Result<(), anyhow::Error> {
    self
      .client
      .stop_container(
        name,
        Some(StopContainerOptions {
          t: 120,
          ..Default::default()
        }),
      )
      .await
      .map_err(|e| anyhow::anyhow!(e))
  }
}
