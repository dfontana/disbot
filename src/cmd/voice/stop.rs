use super::SubCommandHandler;
use crate::cmd::{arg_util::Args, voice::connect_util::DisconnectMessage};
use derive_new::new;
use kitchen_sink::actor::ActorHandle;
use serenity::{all::CommandInteraction, async_trait, client::Context};
use tracing::info;

#[derive(new)]
pub struct Stop {
  disconnect: ActorHandle<DisconnectMessage>,
}

#[async_trait]
impl SubCommandHandler for Stop {
  async fn handle(
    &self,
    _ctx: &Context,
    _itx: &CommandInteraction,
    _: &Args,
  ) -> Result<(), anyhow::Error> {
    info!("Stopping voice playback");
    let _ = self
      .disconnect
      .send(DisconnectMessage::Disconnect(true))
      .await;
    Ok(())
  }
}
