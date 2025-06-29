use anyhow::Result;
use async_trait::async_trait;
use serenity::futures::future;
use tokio::{signal, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument};

#[async_trait]
pub trait ShutdownHook: Send + Sync {
  async fn shutdown(&self) -> Result<()> {
    Ok(())
  }
}

pub struct ShutdownCoordinator {
  token: CancellationToken,
  tasks: Vec<JoinHandle<()>>,
}

impl ShutdownCoordinator {
  pub fn new() -> Self {
    Self {
      token: CancellationToken::new(),
      tasks: Vec::new(),
    }
  }

  pub fn token(&self) -> CancellationToken {
    self.token.clone()
  }

  pub fn register_task(&mut self, task: JoinHandle<()>) {
    self.tasks.push(task);
  }

  #[instrument(name = "Shutdown", level = "INFO", skip(self))]
  pub async fn wait_for_shutdown(self) {
    #[cfg(unix)]
    {
      let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
        .expect("Failed to install SIGINT handler");
      let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Failed to install SIGTERM handler");

      tokio::select! {
          _ = sigint.recv() => {
              info!("Received SIGINT, initiating graceful shutdown");
          }
          _ = sigterm.recv() => {
              info!("Received SIGTERM, initiating graceful shutdown");
          }
          _ = self.token.cancelled() => {
              info!("Shutdown requested programmatically");
          }
      }
    }

    #[cfg(windows)]
    {
      tokio::select! {
          _ = signal::ctrl_c() => {
              info!("Received Ctrl+C, initiating graceful shutdown");
          }
          _ = self.token.cancelled() => {
              info!("Shutdown requested programmatically");
          }
      }
    }

    info!("Starting graceful shutdown sequence");

    // Cancel the token to signal all tasks to shutdown and wait for them to do so
    self.token.cancel();
    for res in future::join_all(self.tasks).await {
      if let Err(e) = res {
        error!("Shutdown hook failed: {}", e);
      }
    }

    info!("Graceful shutdown sequence completed");
  }
}
