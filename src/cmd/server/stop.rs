use crate::{
  cmd::{arg_util::Args, SubCommandHandler},
  docker::Docker,
};
use anyhow::anyhow;
use derive_new::new;
use serenity::{
  all::CommandInteraction, async_trait, builder::EditInteractionResponse, prelude::Context,
};

// Helper function to send response
async fn send_response(
  ctx: &Context,
  itx: &CommandInteraction,
  message: String,
) -> Result<(), anyhow::Error> {
  itx
    .edit_response(&ctx.http, EditInteractionResponse::new().content(message))
    .await?;
  Ok(())
}

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

    match self.docker.stop(name).await {
      Ok(_) => send_response(ctx, itx, "Server stopped".to_string()).await,
      Err(e) => send_response(ctx, itx, format!("{}", e)).await,
    }
  }
}
