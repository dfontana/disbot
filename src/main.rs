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
mod logging;
mod persistence;
mod shutdown;
mod web;

use clap::Parser;
use cmd::Handler;
use config::Config;
use env::Environment;
use persistence::PersistentStore;
use serenity::{
  client::Client,
  prelude::{GatewayIntents, TypeMapKey},
};
use shutdown::ShutdownCoordinator;
use songbird::SerenityInit;
use std::sync::Arc;
use std::{path::PathBuf, str::FromStr};
use tracing::{error, info, Level};

#[derive(Debug, Clone)]
pub enum WebBindAddress {
  Lan,
  Ip(String),
}

impl std::str::FromStr for WebBindAddress {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "lan" => Ok(WebBindAddress::Lan),
      ip => Ok(WebBindAddress::Ip(ip.to_string())),
    }
  }
}

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

  /// Web server bind address (IP address, "lan" for LAN IP, or "0.0.0.0" for all interfaces)
  #[arg(long, default_value = "0.0.0.0")]
  web_bind_address: WebBindAddress,
}

pub struct HttpClient;
impl TypeMapKey for HttpClient {
  type Value = reqwest::Client;
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
  logging::initalize_logging();

  // Parse CLI arguments with clap
  let cli = Cli::parse();

  // Determine config file path
  let final_config_path = cli
    .config
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| cli.environment.as_toml_file());

  // Load configuration from TOML file
  info!("Loading configuration from {}", final_config_path);
  let config = Config::from_toml(&final_config_path, cli.environment)?;
  if let Ok(mut inst) = Config::global_instance().write() {
    *inst = config.clone();
  }

  // Upgrade logger after bootstrap
  logging::set_log_level(Level::from_str(&config.log_level)?)?;

  // Initialize persistence store
  // Persistence restoration happens in the ready event handler where actor handles are available
  let persistence = Arc::new(PersistentStore::new(&config.db_path)?);

  // Create shutdown coordinator
  let mut shutdown_coordinator = ShutdownCoordinator::new();
  let shutdown_token = shutdown_coordinator.token();

  // Create local chat client
  let chat_client = LocalClient::new(&config, persistence.clone(), shutdown_token.clone()).await?;

  // Create a wrapper for the shutdown hook since LocalClient is not Clone
  struct LocalClientShutdownHook {
    persistence: Arc<PersistentStore>,
  }

  #[async_trait::async_trait]
  impl shutdown::ShutdownHook for LocalClientShutdownHook {
    async fn shutdown(&self) -> anyhow::Result<()> {
      info!("Shutting down LocalClient via shutdown hook");
      // The actual LocalClient shutdown happens in the Discord client shutdown
      // This is just a placeholder for any additional cleanup needed
      Ok(())
    }
  }

  // Register the shutdown hook
  shutdown_coordinator.register_hook(Arc::new(LocalClientShutdownHook {
    persistence: persistence.clone(),
  }));

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
    persistence.clone(),
    shutdown_token.clone(),
  ))
  .application_id(config.app_id.into())
  .await?;

  // Start web server and Discord client concurrently
  let web_server = web::start_server(
    final_config_path,
    persistence.clone(),
    cli.web_bind_address,
    cli.port,
    Some(shutdown_token.clone()),
  );
  let discord_client = client.start();
  let shutdown_listener = shutdown_coordinator.wait_for_shutdown();

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
    _ = shutdown_listener => {
      info!("Shutdown signal received, stopping services");
    }
  };

  info!("Application shutdown complete");
  Ok(())
}
