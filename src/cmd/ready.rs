use serenity::{model::gateway::Ready, prelude::Context};
use tracing::{info, instrument};

pub struct ReadyHandler {}

impl ReadyHandler {
  pub fn new() -> Self {
    ReadyHandler {}
  }

  #[instrument(name = "Ready", level = "INFO", skip(self, ready))]
  pub async fn ready(&self, _: &Context, ready: &Ready) {
    info!("{} is connected!", ready.user.name);
  }
}
