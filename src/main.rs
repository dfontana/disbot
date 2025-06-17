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
mod web;

use std::str::FromStr;

use docker::Docker;
use serenity::{
  client::Client,
  prelude::{GatewayIntents, TypeMapKey},
};
use songbird::SerenityInit;
use tracing::{error, Level};
use tracing_subscriber::{filter, prelude::*, reload, Registry};

use cmd::Handler;
use config::Config;
use env::Environment;

// Global handle for runtime log level changes
static LOG_RELOAD_HANDLE: once_cell::sync::Lazy<
  std::sync::Mutex<Option<reload::Handle<filter::LevelFilter, Registry>>>,
> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(None));

pub fn set_log_level(level: Level) -> Result<(), String> {
  let handle_guard = LOG_RELOAD_HANDLE
    .lock()
    .map_err(|e| format!("Lock error: {}", e))?;
  if let Some(handle) = handle_guard.as_ref() {
    handle
      .modify(|filter| *filter = filter::LevelFilter::from_level(level))
      .map_err(|e| format!("Failed to update log level: {}", e))?;
    Ok(())
  } else {
    Err("Log reload handle not initialized".to_string())
  }
}

pub struct HttpClient;
impl TypeMapKey for HttpClient {
  type Value = reqwest::Client;
}

#[tokio::main]
async fn main() {
  let args: Vec<String> = std::env::args().collect();

  // Parse CLI arguments
  let mut env = Environment::default();
  let mut config_path: Option<String> = None;
  let mut web_port = 3450u16;

  let mut i = 1;
  while i < args.len() {
    match args[i].as_str() {
      "--config" | "-c" => {
        if i + 1 < args.len() {
          config_path = Some(args[i + 1].clone());
          i += 2;
        } else {
          eprintln!("Error: --config requires a file path");
          std::process::exit(1);
        }
      }
      "--port" | "-p" => {
        if i + 1 < args.len() {
          web_port = args[i + 1].parse().unwrap_or_else(|_| {
            eprintln!("Error: Invalid port number");
            std::process::exit(1);
          });
          i += 2;
        } else {
          eprintln!("Error: --port requires a port number");
          std::process::exit(1);
        }
      }
      "prod" | "dev" => {
        env = Environment::from_str(&args[i]).unwrap_or_else(|_| {
          eprintln!("Error: Invalid environment");
          std::process::exit(1);
        });
        i += 1;
      }
      _ => {
        // Check if it's an environment argument without flag
        if let Ok(parsed_env) = Environment::from_str(&args[i]) {
          env = parsed_env;
        }
        i += 1;
      }
    }
  }

  // Determine config file path
  let final_config_path = config_path.unwrap_or_else(|| env.as_toml_file());

  // Load configuration from TOML file
  println!("Loading configuration from {}", final_config_path);
  let config = match Config::from_toml(&final_config_path, env) {
    Ok(config) => {
      // Update global instance
      if let Ok(mut inst) = Config::global_instance().write() {
        *inst = config.clone();
      }
      config
    }
    Err(e) => {
      eprintln!(
        "Error loading configuration from {}: {}",
        final_config_path, e
      );
      std::process::exit(1);
    }
  };

  // Set up reloadable tracing subscriber
  let initial_level = Level::from_str(&config.log_level).unwrap();
  let (filter, reload_handle) = reload::Layer::new(filter::LevelFilter::from_level(initial_level));

  // Store the reload handle globally
  {
    let mut handle_guard = LOG_RELOAD_HANDLE.lock().unwrap();
    *handle_guard = Some(reload_handle);
  }

  // Initialize subscriber with reloadable filter
  tracing_subscriber::Registry::default()
    .with(filter)
    .with(tracing_subscriber::fmt::Layer::default().with_target(false))
    .init();
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
    Docker::new().unwrap(),
  ))
  .application_id(config.app_id.into())
  .await
  .expect("Err creating client");

  // Start web server and Discord client concurrently
  let web_server = web::start_server(final_config_path, web_port);
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
