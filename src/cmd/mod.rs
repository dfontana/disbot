use crate::config::Config;
use serenity::{
  async_trait,
  model::{
    channel::{Message, Reaction},
    gateway::Ready,
  },
  prelude::*,
};

pub mod dice_roll;
pub mod help;
pub mod poll;
mod ready;
mod reddit_prev;
pub mod server;
mod shrug;

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
      shrug: shrug::ShrugHandler::new(config.clone()),
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

  async fn reaction_add(&self, ctx: Context, react: Reaction) {
    self.poller.add_vote(&ctx, &react).await
  }

  async fn reaction_remove(&self, ctx: Context, react: Reaction) {
    self.poller.remove_vote(&ctx, &react).await
  }
}
