use std::error::Error;

use crate::{actor::ActorHandle, cmd::voice::connect_util::DisconnectMessage};

use super::SubCommandHandler;
use derive_new::new;
use serenity::{
  async_trait,
  client::Context,
  model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption,
  },
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
    _itx: &ApplicationCommandInteraction,
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
