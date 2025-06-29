use anyhow::{anyhow, bail};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tracing::Level;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
  filter::LevelFilter,
  fmt::Layer,
  prelude::*,
  reload::{self, Handle},
  Registry,
};

// Global handle for runtime log level changes
static LOG_RELOAD_HANDLE: Lazy<Mutex<Option<Handle<LevelFilter, Registry>>>> =
  Lazy::new(|| Mutex::new(None));

pub fn set_log_level(level: Level) -> Result<(), anyhow::Error> {
  let handle_guard = LOG_RELOAD_HANDLE
    .lock()
    .map_err(|e| anyhow!("Lock error: {}", e))?;
  if let Some(handle) = handle_guard.as_ref() {
    handle
      .modify(|filter| *filter = LevelFilter::from_level(level))
      .map_err(|e| anyhow!("Failed to update log level: {}", e))?;
    Ok(())
  } else {
    bail!("Log reload handle not initialized")
  }
}

pub fn initalize_logging() {
  let (filter, reload_handle) = reload::Layer::new(LevelFilter::from_level(Level::INFO));
  {
    let mut handle_guard = LOG_RELOAD_HANDLE.lock().unwrap();
    *handle_guard = Some(reload_handle);
  }
  tracing_subscriber::Registry::default()
    .with(filter)
    .with(Layer::default().with_target(false))
    .with(ErrorLayer::default())
    .init();
}
