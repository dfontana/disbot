use crate::config::Config;
use serenity::{
  async_trait,
  model::{channel::Message, gateway::Ready, interactions::Interaction},
  prelude::*,
};

pub mod dice_roll;
pub mod help;
pub mod poll;
mod ready;
mod reddit_prev;
pub mod server;
mod shrug;
pub mod voice;

pub struct Handler {
  ready: ready::ReadyHandler,
  shrug: shrug::ShrugHandler,
  reddit: reddit_prev::RedditPreviewHandler,
  poller: poll::PollHandler,
}

impl Handler {
  pub fn new(config: Config) -> Self {
    Handler {
      ready: ready::ReadyHandler::new(),
      shrug: shrug::ShrugHandler::new(config),
      reddit: reddit_prev::RedditPreviewHandler::new(),
      poller: poll::PollHandler::new(),
    }
  }
}

#[async_trait]
impl EventHandler for Handler {
  async fn message(&self, ctx: Context, msg: Message) {
    tokio::join!(
      self.shrug.message(&ctx, &msg),
      self.reddit.message(&ctx, &msg),
    );
  }

  async fn ready(&self, ctx: Context, rdy: Ready) {
    self.ready.ready(&ctx, &rdy).await
  }

  async fn interaction_create(&self, ctx: Context, itx: Interaction) {
    match itx {
      Interaction::MessageComponent(d) => self.poller.handle(&ctx, d).await,
      _ => (),
    }
  }
}
