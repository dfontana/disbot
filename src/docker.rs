use anyhow::anyhow;
use async_trait::async_trait;
use bollard::{
  query_parameters::{ListContainersOptions, StopContainerOptions},
  service::{ContainerStateStatusEnum, ContainerSummary},
};
use std::collections::HashMap;
use tracing::warn;

#[async_trait]
pub trait DockerClient: Send + Sync {
  async fn list(&self) -> Result<Vec<ContainerSummary>, anyhow::Error>;
  async fn status(&self, name: &str) -> Result<ContainerStateStatusEnum, anyhow::Error>;
  async fn start(&self, name: &str) -> Result<(), anyhow::Error>;
  async fn stop(&self, name: &str) -> Result<(), anyhow::Error>;
}

#[derive(Clone)]
pub struct BollardDocker {
  client: bollard::Docker,
}

impl BollardDocker {
  pub fn new() -> Result<BollardDocker, anyhow::Error> {
    Ok(BollardDocker {
      client: bollard::Docker::connect_with_socket_defaults()?,
    })
  }
}

#[async_trait]
impl DockerClient for BollardDocker {
  async fn list(&self) -> Result<Vec<ContainerSummary>, anyhow::Error> {
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
        filters: Some(list_container_filters),
        ..Default::default()
      }))
      .await
      .map_err(|e| anyhow!(e))
  }

  async fn status(&self, name: &str) -> Result<ContainerStateStatusEnum, anyhow::Error> {
    self
      .client
      .inspect_container(
        name,
        None::<bollard::query_parameters::InspectContainerOptions>,
      )
      .await
      .map_err(|e| anyhow!(e))
      .and_then(|res| {
        res
          .state
          .and_then(|s| s.status)
          .ok_or(anyhow!("Container in Unknown State"))
      })
  }

  async fn start(&self, name: &str) -> Result<(), anyhow::Error> {
    self
      .client
      .start_container(
        name,
        None::<bollard::query_parameters::StartContainerOptions>,
      )
      .await
      .map_err(|e| anyhow!(e))
  }

  async fn stop(&self, name: &str) -> Result<(), anyhow::Error> {
    self
      .client
      .stop_container(
        name,
        Some(StopContainerOptions {
          t: Some(120),
          ..Default::default()
        }),
      )
      .await
      .map_err(|e| anyhow!(e))
  }
}

pub struct NoOpDocker;

#[async_trait]
impl DockerClient for NoOpDocker {
  async fn list(&self) -> Result<Vec<ContainerSummary>, anyhow::Error> {
    warn!("Docker unavailable: list operation attempted");
    Err(anyhow!("Docker is not available"))
  }

  async fn status(&self, name: &str) -> Result<ContainerStateStatusEnum, anyhow::Error> {
    warn!(
      "Docker unavailable: status operation attempted for {}",
      name
    );
    Err(anyhow!("Docker is not available"))
  }

  async fn start(&self, name: &str) -> Result<(), anyhow::Error> {
    warn!("Docker unavailable: start operation attempted for {}", name);
    Err(anyhow!("Docker is not available"))
  }

  async fn stop(&self, name: &str) -> Result<(), anyhow::Error> {
    warn!("Docker unavailable: stop operation attempted for {}", name);
    Err(anyhow!("Docker is not available"))
  }
}

pub fn create_docker_client() -> Box<dyn DockerClient> {
  match BollardDocker::new() {
    Ok(docker) => {
      tracing::info!("Docker client connected successfully");
      Box::new(docker)
    }
    Err(e) => {
      warn!(
        "Failed to connect to Docker, using no-op implementation: {}",
        e
      );
      Box::new(NoOpDocker)
    }
  }
}
