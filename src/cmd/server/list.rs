use crate::{cmd::server::wol::Wol, docker::Docker};
use serenity::{
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
};
use tracing::{error, info, instrument};

#[command]
#[description = "List the servers that can be turned on"]
#[usage = "list"]
#[example = "list"]
async fn list(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
  exec_list(ctx, msg).await
}

#[instrument(name = "ServerList", level = "INFO", skip(ctx, msg))]
async fn exec_list(ctx: &Context, msg: &Message) -> CommandResult {
  match Wol::inst()?.ensure_awake() {
    Err(err) => {
      info!("error {:?}", err);
      msg.reply_ping(&ctx.http, err).await?;
      return Ok(());
    }
    _ => (),
  };

  let docker = Docker::client()?.containers();

  let containers = match docker.list().await {
    Ok(c) => c,
    Err(err) => {
      error!("Failed to get containers - {:?}", err);
      return Ok(());
    }
  };

  for c in &containers {
    info!("Data on hand - {:?}", c);
    match docker.inspect(&c.id).await {
      Ok(v) => {
        info!("Inspected data - {:?}", v);
      }
      Err(err) => {
        error!("Failed to inspect container - {:?}", err);
        return Ok(());
      }
    }
  }

  // let id = &containers.get(0).unwrap().id;
  // match docker.start(&id).await {
  //   Err(err) => {
  //     return Ok(());
  //   },
  //   _ => (),
  // }
  Ok(())
}
