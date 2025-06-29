use self::{
  arg_util::Args,
  check_in::{CheckInActor, CheckInMessage},
  poll::{PollActor, PollMessage},
};
use crate::{
  actor::ActorHandle, config::Config, docker::DockerClient, emoji::EmojiLookup,
  persistence::PersistentStore,
};
use itertools::Itertools;
use reqwest::Client;
use serenity::{
  all::{CommandInteraction, ComponentInteraction, Interaction},
  async_trait,
  builder::CreateCommand,
  futures::future,
  model::{channel::Message, gateway::Ready},
  prelude::*,
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::error;

mod arg_util;
pub mod check_in;
mod dice_roll;
pub mod poll;
mod ready;
mod reddit_prev;
mod server;
mod shrug;
mod voice;

#[async_trait]
trait MessageListener: Send + Sync {
  async fn message(&self, ctx: &Context, msg: &Message) -> Result<(), anyhow::Error>;
}

#[async_trait]
trait AppInteractor: Send + Sync {
  fn commands(&self) -> Vec<CreateCommand>;
  async fn app_interact(&self, ctx: &Context, itx: &CommandInteraction);
  async fn msg_interact(&self, _: &Context, _: &ComponentInteraction) {
    // Default is no-op
  }
}

#[async_trait]
trait SubCommandHandler: Send + Sync {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
    args: &Args,
  ) -> Result<(), anyhow::Error>;
}

pub struct Handler {
  listeners: Vec<Box<dyn MessageListener>>,
  app_interactors: Vec<Box<dyn AppInteractor>>,
  ready: ready::ReadyHandler,
}

impl Handler {
  pub fn new(
    config: Config,
    emoji: EmojiLookup,
    http: Client,
    docker: Box<dyn DockerClient>,
    persistence: Arc<PersistentStore>,
    shutdown_token: CancellationToken,
  ) -> Self {
    let poll_handle = ActorHandle::<PollMessage>::spawn_with_shutdown(
      |r, h| PollActor::new(r, h, persistence.clone(), shutdown_token.clone()),
      shutdown_token.clone(),
    );

    let chk_handle = ActorHandle::<CheckInMessage>::spawn_with_shutdown(
      |r, h| {
        Box::new(CheckInActor::new(
          h,
          r,
          poll_handle.clone(),
          persistence.clone(),
        ))
      },
      shutdown_token.clone(),
    );
    Handler {
      listeners: vec![
        Box::new(shrug::ShrugHandler::new(config.clone(), emoji.clone())),
        Box::new(reddit_prev::RedditPreviewHandler::new(http.clone())),
      ],
      app_interactors: vec![
        Box::new(poll::Poll::new(emoji.clone(), poll_handle.clone())),
        Box::new(check_in::CheckIn::new(emoji.clone(), chk_handle.clone())),
        Box::new(dice_roll::DiceRoll::new(emoji.clone())),
        Box::new(voice::Voice::new(config, emoji.clone())),
        Box::new(server::GameServers::new(emoji, http, docker)),
      ],
      ready: ready::ReadyHandler::new(poll_handle, chk_handle),
    }
  }
}

#[async_trait]
impl EventHandler for Handler {
  async fn message(&self, ctx: Context, msg: Message) {
    let results = future::join_all(self.listeners.iter().map(|f| f.message(&ctx, &msg))).await;
    for res in results {
      if let Err(e) = res {
        error!("{}", e);
      }
    }
  }

  async fn ready(&self, ctx: Context, rdy: Ready) {
    // Register Slash commands with each guild the bot is connected to
    for guild_id in ctx.cache.guilds() {
      guild_id
        .set_commands(
          &ctx.http,
          self
            .app_interactors
            .iter()
            .flat_map(|ai| ai.commands().into_iter())
            .collect_vec(),
        )
        .await
        .expect("Failed to Register Application Context");
    }
    self.ready.ready(&ctx, &rdy).await;
  }

  async fn interaction_create(&self, ctx: Context, itx: Interaction) {
    match itx {
      Interaction::Component(d) => {
        future::join_all(
          self
            .app_interactors
            .iter()
            .map(|f| f.msg_interact(&ctx, &d)),
        )
        .await;
      }
      Interaction::Command(d) => {
        future::join_all(
          self
            .app_interactors
            .iter()
            .map(|f| f.app_interact(&ctx, &d)),
        )
        .await;
      }
      _ => (),
    }
  }
}
