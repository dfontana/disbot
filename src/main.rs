extern crate hex;
extern crate rand;
extern crate regex;
extern crate reqwest;

mod actor;
mod cmd;
mod config;
mod docker;
mod emoji;
mod env;
mod persistence;
mod web;

use std::{path::PathBuf, str::FromStr, sync::Mutex};

use clap::Parser;
use once_cell::sync::Lazy;
use serenity::{
  client::Client,
  prelude::{GatewayIntents, TypeMapKey},
};
use songbird::SerenityInit;
use tracing::{error, info, Level};
use tracing_subscriber::{
  filter::LevelFilter,
  fmt::Layer,
  prelude::*,
  reload::{self, Handle},
  Registry,
};

use cmd::Handler;
use config::Config;
use env::Environment;
use persistence::PersistentStore;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "disbot")]
#[command(about = "Discord bot with admin web interface")]
struct Cli {
  /// Environment to run in
  #[arg(value_enum, default_value = "dev")]
  environment: Environment,

  /// Custom configuration file path
  #[arg(short, long)]
  config: Option<PathBuf>,

  /// Web server port
  #[arg(short, long, default_value = "3450")]
  port: u16,
}

// Global handle for runtime log level changes
static LOG_RELOAD_HANDLE: Lazy<Mutex<Option<Handle<LevelFilter, Registry>>>> =
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

fn initalize_logging() {
  let (filter, reload_handle) = reload::Layer::new(LevelFilter::from_level(Level::INFO));
  {
    let mut handle_guard = LOG_RELOAD_HANDLE.lock().unwrap();
    *handle_guard = Some(reload_handle);
  }
  tracing_subscriber::Registry::default()
    .with(filter)
    .with(Layer::default().with_target(false))
    .init();
}

pub struct HttpClient;
impl TypeMapKey for HttpClient {
  type Value = reqwest::Client;
}

#[tokio::main]
async fn main() {
  initalize_logging();

  // Parse CLI arguments with clap
  let cli = Cli::parse();

  // Determine config file path
  let final_config_path = cli
    .config
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| cli.environment.as_toml_file());

  // Load configuration from TOML file
  info!("Loading configuration from {}", final_config_path);
  let config = match Config::from_toml(&final_config_path, cli.environment) {
    Ok(config) => {
      // Update global instance
      if let Ok(mut inst) = Config::global_instance().write() {
        *inst = config.clone();
      }
      config
    }
    Err(e) => {
      error!(
        "Error loading configuration from {}: {}",
        final_config_path, e
      );
      std::process::exit(1);
    }
  };

  // Upgrade logger after bootstrap
  if let Err(e) = set_log_level(Level::from_str(&config.log_level).unwrap()) {
    error!(
      "Failed to update logger level from config {}: {}",
      &config.log_level, e
    );
    std::process::exit(1);
  }

  // Initialize persistence store
  let persistence = match PersistentStore::new(&config.db_path) {
    Ok(store) => Arc::new(store),
    Err(e) => {
      error!("Failed to initialize persistence store: {}", e);
      std::process::exit(1);
    }
  };

  let emoji = emoji::EmojiLookup::new(&config);
  let http = reqwest::Client::new();

  let mut client = Client::builder(
    &config.api_key,
    GatewayIntents::GUILDS
      | GatewayIntents::GUILD_EMOJIS_AND_STICKERS
      | GatewayIntents::GUILD_MESSAGES
      | GatewayIntents::MESSAGE_CONTENT
      | GatewayIntents::GUILD_MESSAGE_REACTIONS
      | GatewayIntents::GUILD_VOICE_STATES,
  )
  .register_songbird()
  .type_map_insert::<HttpClient>(http.clone())
  .event_handler(Handler::new(
    config.clone(),
    emoji,
    http,
    docker::create_docker_client(),
    persistence,
  ))
  .application_id(config.app_id.into())
  .await
  .unwrap_or_else(|e| {
    error!("Error creating Discord client: {:?}", e);
    std::process::exit(1);
  });

  // Persistence restoration happens in the ready event handler where actor handles are available

  // Start web server and Discord client concurrently
  let web_server = web::start_server(final_config_path, cli.port);
  let discord_client = client.start();

  tokio::select! {
    result = web_server => {
      if let Err(why) = result {
        error!("Failed to start web server: {:?}", why);
      }
    }
    result = discord_client => {
      if let Err(why) = result {
        error!("Failed to start Discord Client: {:?}", why);
      }
    }
  }
}
