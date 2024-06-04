use std::collections::HashMap;

use bollard::{
  container::{ListContainersOptions, StopContainerOptions},
  service::ContainerSummary,
};

pub struct Docker {
  client: bollard::Docker,
}

impl Docker {
  pub fn client() -> Result<Docker, anyhow::Error> {
    Ok(Docker {
      client: bollard::Docker::connect_with_socket_defaults()?,
    })
  }

  pub async fn list(&self) -> Result<Vec<ContainerSummary>, anyhow::Error> {
    let mut list_container_filters: HashMap<String, Vec<String>> = HashMap::new();
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
