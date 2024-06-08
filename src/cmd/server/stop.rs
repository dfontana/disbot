use crate::{
  cmd::{arg_util::Args, SubCommandHandler},
  docker::Docker,
};
use anyhow::anyhow;
use derive_new::new;
use serenity::{
  all::CommandInteraction, async_trait, builder::EditInteractionResponse, prelude::Context,
};

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
    args: &Args,
  ) -> Result<(), anyhow::Error> {
    let name = args
      .str("server-name")
      .map_err(|e| anyhow!("Must provide a server name").context(e))?;

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
