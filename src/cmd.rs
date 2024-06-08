use self::{
  arg_util::Args,
  check_in::{CheckInActor, CheckInMessage},
  poll::{PollActor, PollMessage},
};
use crate::{actor::ActorHandle, config::Config, docker::Docker, emoji::EmojiLookup};
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

mod arg_util;
mod check_in;
mod dice_roll;
mod poll;
mod ready;
mod reddit_prev;
mod server;
mod shrug;
mod voice;

#[async_trait]
trait MessageListener: Send + Sync {
  async fn message(&self, ctx: &Context, msg: &Message);
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
  pub fn new(config: Config, emoji: EmojiLookup, http: Client, docker: Docker) -> Self {
    let poll_handle = ActorHandle::<PollMessage>::spawn(|r, h| PollActor::new(r, h));
    let chk_handle = ActorHandle::<CheckInMessage>::spawn(|r, h| {
      Box::new(CheckInActor::new(h, r, poll_handle.clone()))
    });
    Handler {
      listeners: vec![
        Box::new(shrug::ShrugHandler::new(config.clone(), emoji.clone())),
        Box::new(reddit_prev::RedditPreviewHandler::new(http.clone())),
      ],
      app_interactors: vec![
        Box::new(poll::Poll::new(emoji.clone(), poll_handle)),
        Box::new(check_in::CheckIn::new(emoji.clone(), chk_handle)),
        Box::new(dice_roll::DiceRoll::new(emoji.clone())),
        Box::new(voice::Voice::new(config, emoji.clone())),
        Box::new(server::GameServers::new(emoji, http, docker)),
      ],
      ready: ready::ReadyHandler::default(),
    }
  }
}

#[async_trait]
impl EventHandler for Handler {
  async fn message(&self, ctx: Context, msg: Message) {
    future::join_all(self.listeners.iter().map(|f| f.message(&ctx, &msg))).await;
  }

  async fn ready(&self, ctx: Context, rdy: Ready) {
    // Register Slash commands with each guild that Shibba is connected to
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
