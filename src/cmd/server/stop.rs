use derive_new::new;
use serenity::{
  async_trait,
  model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
  },
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
    itx: &ApplicationCommandInteraction,
    subopt: &CommandDataOption,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let args: HashMap<String, _> = subopt
      .options
      .iter()
      .map(|d| (d.name.to_owned(), d.resolved.to_owned()))
      .collect();

    let name = args
      .get("server-name")
      .and_then(|v| v.to_owned())
      .and_then(|d| match d {
        CommandDataOptionValue::String(v) => Some(v),
        _ => None,
      })
      .ok_or("Must provide a server name")?;

    let msg = match self.docker.stop(&name).await {
      Ok(_) => "Server stopped".into(),
      Err(e) => format!("Failed to list docker containers: {}", e),
    };
    itx
      .edit_original_interaction_response(&ctx.http, |f| f.content(msg))
      .await?;
    Ok(())
  }
}
