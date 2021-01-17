extern crate dotenv;

mod cmd;
mod config;
mod env;

use std::str::FromStr;

use serenity::{client::bridge::gateway::GatewayIntents, prelude::*};

use cmd::ShrugHandler;
use config::Config;
use env::Environment;

#[tokio::main]
async fn main() {
  let env = std::env::args().nth(1).map_or(Environment::default(), |v| {
    Environment::from_str(&v).unwrap()
  });
  dotenv::from_filename(env.as_file()).ok();
  let config = Config::new().expect("Err parsing environment");

  let mut client = Client::builder(&config.get_api_key())
    .intents(
      GatewayIntents::GUILDS
        | GatewayIntents::GUILD_EMOJIS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS,
    )
    .event_handler(ShrugHandler::new(config.clone()))
    .await
    .expect("Err creating client");

  if let Err(why) = client.start().await {
    println!("Client error: {:?}", why);
  }
}
