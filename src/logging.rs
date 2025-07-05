use anyhow::{anyhow, bail};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tracing::Level;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
  filter::{self, Targets},
  fmt::Layer,
  prelude::*,
  reload::{self, Handle},
  Registry,
};

// Global handle for runtime log level changes
static LOG_RELOAD_HANDLE: Lazy<Mutex<Option<Handle<Targets, Registry>>>> =
  Lazy::new(|| Mutex::new(None));

fn mk_targets(level: Level) -> Targets {
  filter::Targets::new()
    .with_target("tokio", Level::TRACE)
    .with_target("runtime", Level::TRACE)
    .with_default(level)
}

pub fn set_log_level(level: Level) -> Result<(), anyhow::Error> {
  let handle_guard = LOG_RELOAD_HANDLE
    .lock()
    .map_err(|e| anyhow!("Lock error: {}", e))?;
  if let Some(handle) = handle_guard.as_ref() {
    handle
      .modify(|filter| *filter = mk_targets(level))
      .map_err(|e| anyhow!("Failed to update log level: {}", e))?;
    Ok(())
  } else {
    bail!("Log reload handle not initialized")
  }
}

pub fn initalize_logging() {
  let (filter, reload_handle) = reload::Layer::new(mk_targets(Level::INFO));
  {
    let mut handle_guard = LOG_RELOAD_HANDLE.lock().unwrap();
    *handle_guard = Some(reload_handle);
  }
  // TODO: Can I have these diagnostics be emitted conditionally? Or maybe skip stdout?
  let console_layer = console_subscriber::spawn();
  tracing_subscriber::Registry::default()
    .with(filter)
    .with(console_layer)
    .with(Layer::default().with_target(false))
    .with(ErrorLayer::default())
    .init();
}
