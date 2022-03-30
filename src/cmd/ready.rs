use serenity::{model::gateway::Ready, prelude::Context};
use tracing::{info, instrument};

#[derive(Default)]
pub struct ReadyHandler {}

impl ReadyHandler {
  #[instrument(name = "Ready", level = "INFO", skip(self, ready))]
  pub async fn ready(&self, _: &Context, ready: &Ready) {
    info!("{} is connected!", ready.user.name);
  }
}
