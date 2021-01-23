use serenity::{model::gateway::Ready, prelude::Context};

pub struct ReadyHandler {}

impl ReadyHandler {
  pub fn new() -> Self {
    ReadyHandler {}
  }

  pub async fn ready(&self, _: &Context, ready: &Ready) {
    println!("{} is connected!", ready.user.name);
  }
}
