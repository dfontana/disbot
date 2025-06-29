use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

#[async_trait]
pub trait ShutdownHook: Send + Sync {
  async fn shutdown(&self) -> Result<()>;
}

pub struct ShutdownCoordinator {
  token: CancellationToken,
  hooks: Vec<Arc<dyn ShutdownHook>>,
}

impl ShutdownCoordinator {
  pub fn new() -> Self {
    Self {
      token: CancellationToken::new(),
      hooks: Vec::new(),
    }
  }

  pub fn token(&self) -> CancellationToken {
    self.token.clone()
  }

  pub fn register_hook(&mut self, hook: Arc<dyn ShutdownHook>) {
    self.hooks.push(hook);
  }

  pub async fn wait_for_shutdown(&self) {
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

    self.trigger_shutdown().await;
  }

  pub async fn trigger_shutdown(&self) {
    info!("Starting graceful shutdown sequence");

    // Cancel the token to signal all tasks to shutdown
    self.token.cancel();

    // Execute all shutdown hooks
    for (i, hook) in self.hooks.iter().enumerate() {
      match hook.shutdown().await {
        Ok(_) => info!("Shutdown hook {} completed successfully", i),
        Err(e) => error!("Shutdown hook {} failed: {}", i, e),
      }
    }

    info!("Graceful shutdown sequence completed");
  }

  pub fn shutdown_signal(&self) -> CancellationToken {
    self.token.clone()
  }
}

impl Default for ShutdownCoordinator {
  fn default() -> Self {
    Self::new()
  }
}
