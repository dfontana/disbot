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
mod debug;
mod emoji;
mod env;

use std::str::FromStr;

use serenity::{
  client::{bridge::gateway::GatewayIntents, Client},
  framework::standard::{macros::group, StandardFramework},
};

use cmd::{dice_roll::*, help::*, poll::*, server::*, Handler};
use config::Config;
use env::Environment;

#[group]
#[description = "Utilities the Sheebs has Graced You With"]
#[summary = "Utilities Sheebs Givith"]
#[commands(roll, poll)]
#[sub_groups(server)]
struct General;

#[tokio::main]
async fn main() {
  let env = std::env::args().nth(1).map_or(Environment::default(), |v| {
    Environment::from_str(&v).unwrap()
  });
  dotenv::from_filename(env.as_file()).ok();
  let config = Config::set(env).expect("Err parsing environment");
  emoji::configure(&config).expect("Failed to setup emoji lookup");
  debug::configure(&config).expect("Failed to setup debug logger");
  let framework = StandardFramework::new()
    .configure(|c| c.prefix("!"))
    .group(&GENERAL_GROUP)
    .help(&HELP);

  let mut client = Client::builder(&config.get_api_key())
    .intents(
      GatewayIntents::GUILDS
        | GatewayIntents::GUILD_EMOJIS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS,
    )
    .framework(framework)
    .event_handler(Handler::new(config.clone()))
    .await
    .expect("Err creating client");

  if let Err(why) = client.start().await {
    println!("Client error: {:?}", why);
  }
}
