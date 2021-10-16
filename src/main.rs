#[macro_use]
extern crate derive_builder;
extern crate dotenv;
extern crate hex;
#[macro_use]
extern crate lazy_static;
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

use serenity::{
  client::{bridge::gateway::GatewayIntents, Client},
  framework::standard::{macros::group, StandardFramework},
};
use songbird::SerenityInit;
use tracing::{error, Level};

use cmd::{dice_roll::*, help::*, poll::*, server::*, voice::*, Handler};
use config::Config;
use env::Environment;

#[group]
#[description = "Utilities the Sheebs has Graced You With"]
#[summary = "Utilities Sheebs Givith"]
#[commands(roll, poll)]
#[sub_groups(server, voice)]
struct General;

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
  cmd::server::configure(&config.server).expect("Failed to setup game server");
  docker::configure(&config.server).expect("Failed to setup docker for game server");

  let framework = StandardFramework::new()
    .configure(|c| c.prefix("!"))
    .group(&GENERAL_GROUP)
    .help(&HELP);

  let mut client = Client::builder(&config.api_key)
    .intents(
      GatewayIntents::GUILDS
        | GatewayIntents::GUILD_EMOJIS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_VOICE_STATES,
    )
    .framework(framework)
    .register_songbird()
    .event_handler(Handler::new(config.clone()))
    .await
    .expect("Err creating client");

  if let Err(why) = client.start().await {
    error!("Failed to start Discord Client {:?}", why);
  }
}
