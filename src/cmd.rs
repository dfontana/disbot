use std::error::Error;

use crate::{actor::ActorHandle, config::Config, emoji::EmojiLookup};
use serenity::{
  async_trait,
  builder::CreateApplicationCommands,
  futures::future,
  model::{
    channel::Message,
    gateway::Ready,
    id::GuildId,
    prelude::interaction::{
      application_command::{ApplicationCommandInteraction, CommandDataOption},
      message_component::MessageComponentInteraction,
      Interaction,
    },
  },
  prelude::*,
};

use self::{
  check_in::{CheckInActor, CheckInMessage},
  poll::{PollActor, PollMessage},
};

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
  fn register(&self, commands: &mut CreateApplicationCommands);
  async fn app_interact(&self, ctx: &Context, itx: &ApplicationCommandInteraction);
  async fn msg_interact(&self, _: &Context, _: &MessageComponentInteraction) {
    // Default is no-op
  }
}

#[async_trait]
trait SubCommandHandler: Send + Sync {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &ApplicationCommandInteraction,
    subopt: &CommandDataOption,
  ) -> Result<(), Box<dyn Error>>;
}

pub struct Handler {
  listeners: Vec<Box<dyn MessageListener>>,
  app_interactors: Vec<Box<dyn AppInteractor>>,
  ready: ready::ReadyHandler,
}

impl Handler {
  pub fn new(config: Config, emoji: EmojiLookup) -> Self {
    let poll_handle = ActorHandle::<PollMessage>::spawn(|r, h| PollActor::new(r, h));
    let chk_handle = ActorHandle::<CheckInMessage>::spawn(|r, h| {
      Box::new(CheckInActor::new(h, r, poll_handle.clone()))
    });
    Handler {
      listeners: vec![
        Box::new(shrug::ShrugHandler::new(config.clone(), emoji.clone())),
        Box::<reddit_prev::RedditPreviewHandler>::default(),
      ],
      app_interactors: vec![
        Box::new(poll::Poll::new(emoji.clone(), poll_handle)),
        Box::new(check_in::CheckIn::new(emoji.clone(), chk_handle)),
        Box::new(dice_roll::DiceRoll::new(emoji.clone())),
        Box::new(voice::Voice::new(config, emoji)),
        // server::Server::new(config),
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
      GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
        self.app_interactors.iter().for_each(|app| {
          app.register(commands);
        });
        commands
      })
      .await
      .expect("Failed to Register Application Context");
    }

    self.ready.ready(&ctx, &rdy).await;
  }

  async fn interaction_create(&self, ctx: Context, itx: Interaction) {
    match itx {
      Interaction::MessageComponent(d) => {
        future::join_all(
          self
            .app_interactors
            .iter()
            .map(|f| f.msg_interact(&ctx, &d)),
        )
        .await;
      }
      Interaction::ApplicationCommand(d) => {
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
