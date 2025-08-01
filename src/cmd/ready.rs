use super::{check_in::CheckInMessage, poll::PollMessage};
use crate::cmd::CallContext;
use derive_new::new;
use kitchen_sink::actor::ActorHandle;
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

    let cctx = CallContext {
      http: ctx.http.clone(),
    };

    // Restore polls
    self
      .poll_handle
      .send(PollMessage::RestorePolls(cctx.clone()))
      .await;

    // Restore check-in
    self
      .checkin_handle
      .send(CheckInMessage::RestoreConfig(cctx))
      .await;
  }
}
