use crate::cmd::SubCommandHandler;
use derive_new::new;
use serenity::{
  async_trait,
  client::Context,
  model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption,
  },
};
use std::error::Error;

#[derive(new)]
pub struct Start {}

#[async_trait]
impl SubCommandHandler for Start {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &ApplicationCommandInteraction,
    _subopt: &CommandDataOption,
  ) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
  }
}

async fn wrapped_handle(
  start: &Start,
  ctx: &Context,
  itx: &ApplicationCommandInteraction,
  subopt: &CommandDataOption,
) -> Result<(), Box<dyn Error + Send + Sync>> {
  todo!()
}
