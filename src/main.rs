#[macro_use]
extern crate derive_builder;

mod actor;
mod cmd;
mod config;
mod docker;
mod emoji;
mod env;
mod template;

use actor::ActorHandle;
use axum::Router;
use cmd::{CheckInActor, CheckInMessage, Handler, PollActor, PollMessage};
use config::Config;
use env::Environment;
use serenity::{client::Client, prelude::GatewayIntents};
use songbird::SerenityInit;
use std::str::FromStr;
use std::thread;
use tokio::runtime::Handle;
use tower_http::trace::TraceLayer;
use tracing::{error, info, Level};

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
  let emoji = emoji::EmojiLookup::new(&config);
  docker::configure(&config.server).expect("Failed to setup docker for game server");

  let actor_comms = ActorHandles::init();
  let server_actor_comms = actor_comms.clone();

  let handle = Handle::current();
  let server_thread = thread::spawn(move || handle.spawn(spawn_thread(server_actor_comms)));

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
  .event_handler(Handler::new(config.clone(), emoji, actor_comms))
  .application_id(config.app_id)
  .await
  .expect("Err creating client");

  if let Err(why) = client.start().await {
    error!("Failed to start Discord Client {:?}", why);
  }

  server_thread.join().expect("Server Thread Panicked");
}

async fn spawn_thread(actors: ActorHandles) {
  let app = Router::new()
    .nest("/ui", template::admin_routes(actors))
    .layer(TraceLayer::new_for_http());

  info!("Starting server");
  axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
    .serve(app.into_make_service())
    .await
    .unwrap();
  info!("Server started");
}

#[derive(Clone)]
pub struct ActorHandles {
  poll: ActorHandle<PollMessage>,
  chk: ActorHandle<CheckInMessage>,
}

impl ActorHandles {
  pub fn init() -> Self {
    let polls = ActorHandle::<PollMessage>::spawn(|r, h| PollActor::new(r, h));
    ActorHandles {
      chk: ActorHandle::<CheckInMessage>::spawn(|r, h| Box::new(CheckInActor::new(h, r, polls.clone()))),
      poll: polls,
    }
  }
}
