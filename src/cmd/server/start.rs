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

// Helper function to send error response
async fn send_error_response(
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
        return send_error_response(
          ctx,
          itx,
          format!("Server in state that can't be started: {}", s),
        )
        .await;
      }
      Err(e) => {
        return send_error_response(ctx, itx, format!("{}", e)).await;
      }
    }

    match self.docker.start(name).await {
      Ok(_) => send_error_response(ctx, itx, "Server starting".to_string()).await,
      Err(e) => send_error_response(ctx, itx, format!("{}", e)).await,
    }
  }
}
