#[macro_use]
extern crate derive_builder;
extern crate dotenv;
extern crate hex;
extern crate rand;
extern crate regex;
extern crate reqwest;
extern crate select;

mod cmd;
mod config;
mod docker;
mod emoji;
mod env;

use std::str::FromStr;

use serenity::client::{bridge::gateway::GatewayIntents, Client};
use songbird::SerenityInit;
use tracing::{error, Level};

use cmd::Handler;
use config::Config;
use env::Environment;

#[tokio::main]
async fn main() {
  let env = std::env::args()
    .nth(1)
    .or_else(|| std::env::var("RUN_ENV").ok())
    .map_or(Environment::default(), |v| {
      println!("Given '{}' env to run", &v);
      Environment::from_str(&v).unwrap()
    });
  dotenv::from_filename(env.as_file()).ok();
  let config = Config::set(env).expect("Err parsing environment");

  tracing_subscriber::fmt()
    .with_max_level(Level::from_str(&config.log_level).unwrap())
    .with_target(false)
    .init();
  emoji::configure(&config).expect("Failed to setup emoji lookup");
  docker::configure(&config.server).expect("Failed to setup docker for game server");

  let mut client = Client::builder(&config.api_key)
    .intents(
      GatewayIntents::GUILDS
        | GatewayIntents::GUILD_EMOJIS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_VOICE_STATES,
    )
    .register_songbird()
    .event_handler(Handler::new(config.clone()))
    .application_id(config.app_id)
    .await
    .expect("Err creating client");

  if let Err(why) = client.start().await {
    error!("Failed to start Discord Client {:?}", why);
  }
}
