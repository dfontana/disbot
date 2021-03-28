use crate::{cmd::server::wol::Wol, debug::Debug};
use serenity::{
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
};

#[command]
#[description = "Stop the game server"]
#[usage = "stop"]
#[example = "stop"]
async fn stop(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
  let wol = Wol::inst()?;

  let is_awake = match wol.is_awake() {
    Ok(v) => v,
    Err(e) => {
      Debug::inst("server_stop").log(&format!("Failed to check Game Server is awake - {}", e));
      msg
        .reply_ping(&ctx.http, "Couldn't stop the server :(")
        .await?;
      return Ok(());
    }
  };

  if !is_awake {
    msg.reply_ping(&ctx.http, "Server is not awake").await?;
    return Ok(());
  }

  match wol.shutdown() {
    Ok(0) => {
      msg.reply_ping(&ctx.http, "Server is stopping").await?;
    }
    Ok(left) => {
      let msg_res = format!(
        "Stop ran recently, please wait {}m{}s",
        left / 60,
        left % 60
      );
      msg.reply_ping(&ctx.http, msg_res).await?;
    }
    Err(e) => {
      Debug::inst("server_stop").log(&format!("Failed to stop Game Server - {}", e));
      msg
        .reply_ping(&ctx.http, "Couldn't stop the server :(")
        .await?;
      return Ok(());
    }
  }

  Debug::inst("server_start").log("Server has Woken");
  Ok(())
}
