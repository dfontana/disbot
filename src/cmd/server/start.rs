use crate::{cmd::SubCommandHandler, docker::Docker};
use bollard::service::ContainerStateStatusEnum::{CREATED, EXITED};
use derive_new::new;
use serenity::{
  all::{CommandDataOption, CommandDataOptionValue, CommandInteraction},
  async_trait,
  builder::EditInteractionResponse,
  client::Context,
};
use std::collections::HashMap;

#[derive(new)]
pub struct Start {
  docker: Docker,
}

#[async_trait]
impl SubCommandHandler for Start {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
    subopt: &CommandDataOption,
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
        CommandDataOptionValue::String(v) => Some(v),
        _ => None,
      })
      .ok_or("Must provide a server name")?;

    match self.docker.status(&name).await {
      Ok(CREATED | EXITED) => {}
      Ok(s) => {
        itx
          .edit_response(
            &ctx.http,
            EditInteractionResponse::new()
              .content(format!("Server in state that can't be started: {}", s)),
          )
          .await?;
        return Ok(());
      }
      Err(e) => {
        itx
          .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!("{}", e)),
          )
          .await?;
        return Ok(());
      }
    }

    let msg = match self.docker.start(&name).await {
      Ok(_) => "Server starting".into(),
      Err(e) => format!("{}", e),
    };
    itx
      .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
      .await?;
    Ok(())
  }
}
