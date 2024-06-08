use derive_new::new;
use serenity::{
  all::{CommandDataOption, CommandInteraction, ResolvedValue},
  async_trait,
  builder::EditInteractionResponse,
  prelude::Context,
};
use std::collections::HashMap;

use crate::{cmd::SubCommandHandler, docker::Docker};

#[derive(new)]
pub struct Stop {
  docker: Docker,
}

#[async_trait]
impl SubCommandHandler for Stop {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
    _subopt: &CommandDataOption,
  ) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Let's move to autocomplete on these
    let args: HashMap<String, _> = itx
      .data
      .options()
      .iter()
      .map(|d| (d.name.to_owned(), d.value.to_owned()))
      .collect();

    let name = args
      .get("server-name")
      .and_then(|d| match d {
        ResolvedValue::String(v) => Some(v),
        _ => None,
      })
      .ok_or("Must provide a server name")?;

    let msg = match self.docker.stop(&name).await {
      Ok(_) => "Server stopped".into(),
      Err(e) => format!("{}", e),
    };
    itx
      .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
      .await?;
    Ok(())
  }
}
