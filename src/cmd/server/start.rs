use crate::{
  cmd::{arg_util::Args, SubCommandHandler},
  docker::Docker,
};
use anyhow::anyhow;
use bollard::service::ContainerStateStatusEnum::{CREATED, EXITED};
use derive_new::new;
use serenity::{
  all::CommandInteraction, async_trait, builder::EditInteractionResponse, client::Context,
};

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
    args: &Args,
  ) -> Result<(), anyhow::Error> {
    // TODO: Let's move to autocomplete on these
    let name = args
      .str("server-name")
      .map_err(|e| anyhow!("Must provide a server name").context(e))?;

    match self.docker.status(name).await {
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

    let msg = match self.docker.start(name).await {
      Ok(_) => "Server starting".into(),
      Err(e) => format!("{}", e),
    };
    itx
      .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
      .await?;
    Ok(())
  }
}
