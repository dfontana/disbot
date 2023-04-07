use std::error::Error;

use super::{connect_util::DisconnectHandle, SubCommandHandler};
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
  disconnect: DisconnectHandle,
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
    let _ = self.disconnect.stop().await;
    Ok(())
  }
}
