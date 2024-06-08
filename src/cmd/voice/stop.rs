use std::error::Error;

use crate::{actor::ActorHandle, cmd::voice::connect_util::DisconnectMessage};

use super::SubCommandHandler;
use derive_new::new;
use serenity::{
  all::{CommandDataOption, CommandInteraction},
  async_trait,
  client::Context,
};
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
    _subopt: &CommandDataOption,
  ) -> Result<(), Box<dyn Error>> {
    info!("Stopping voice playback");
    let _ = self
      .disconnect
      .send(DisconnectMessage::Disconnect(true))
      .await;
    Ok(())
  }
}
