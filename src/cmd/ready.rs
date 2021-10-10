use serenity::{model::gateway::Ready, prelude::Context};
use tracing::{info, info_span};

pub struct ReadyHandler {}

impl ReadyHandler {
  pub fn new() -> Self {
    ReadyHandler {}
  }

  pub async fn ready(&self, _: &Context, ready: &Ready) {
    let span = info_span!("Ready");
    let _enter = span.enter();
    info!("{} is connected!", ready.user.name);
  }
}
