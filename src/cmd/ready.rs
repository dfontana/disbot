use super::{check_in::CheckInMessage, poll::PollMessage};
use crate::actor::ActorHandle;
use derive_new::new;
use serenity::{model::gateway::Ready, prelude::Context};
use tracing::{info, instrument};

#[derive(new)]
pub struct ReadyHandler {
  poll_handle: ActorHandle<PollMessage>,
  checkin_handle: ActorHandle<CheckInMessage>,
}

impl ReadyHandler {
  #[instrument(name = "ready", level = "INFO", skip(self, ready))]
  pub async fn ready(&self, ctx: &Context, ready: &Ready) {
    info!("{} is connected!", ready.user.name);

    // Restore persisted polls and check-in configurations
    info!("Restoring persistent state...");

    // Restore polls
    self
      .poll_handle
      .send(PollMessage::RestorePolls(ctx.http.clone()))
      .await;

    // Restore check-in
    self
      .checkin_handle
      .send(CheckInMessage::RestoreConfig(ctx.http.clone()))
      .await;
  }
}
