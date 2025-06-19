// Expose modules for integration testing
pub mod actor;
pub mod cmd;
pub mod config;
pub mod docker;
pub mod emoji;
pub mod env;
pub mod persistence;
pub mod web;

use once_cell::sync::Lazy;
use std::sync::Mutex;
use tracing::Level;
use tracing_subscriber::{filter::LevelFilter, reload, Registry};

// Re-export commonly used types for tests
pub use cmd::{
  check_in::{CheckInActor, CheckInCtx, CheckInMessage},
  poll::{pollstate::PollState, PollActor, PollMessage},
};
pub use env::Environment;
pub use persistence::PersistentStore;

// Global handle for runtime log level changes (needed for config module)
static LOG_RELOAD_HANDLE: Lazy<Mutex<Option<reload::Handle<LevelFilter, Registry>>>> =
  Lazy::new(|| Mutex::new(None));

pub fn set_log_level(level: Level) -> Result<(), String> {
  let handle_guard = LOG_RELOAD_HANDLE
    .lock()
    .map_err(|e| format!("Lock error: {}", e))?;
  if let Some(handle) = handle_guard.as_ref() {
    handle
      .modify(|filter| *filter = LevelFilter::from_level(level))
      .map_err(|e| format!("Failed to update log level: {}", e))?;
    Ok(())
  } else {
    Err("Log reload handle not initialized".to_string())
  }
}

// Type definitions for cmd modules
use serenity::prelude::TypeMapKey;

pub struct HttpClient;
impl TypeMapKey for HttpClient {
  type Value = reqwest::Client;
}