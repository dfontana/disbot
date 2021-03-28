use crate::{cmd::server::wol::Wol, debug::Debug, Config};
use serenity::{
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
};

#[command]
#[description = "Wake the game server"]
#[usage = "wake"]
#[example = "wake"]
async fn wake(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
  let cfg = Config::inst()?;
  let wol = Wol::new(&cfg.server)?;

  let is_awake = match wol.is_awake() {
    Ok(v) => v,
    Err(e) => {
      Debug::inst("server_wake").log(&format!("Failed to check Game Server is awake - {}", e));
      msg
        .reply_ping(&ctx.http, "Couldn't wake the server :(")
        .await?;
      return Ok(());
    }
  };

  if is_awake {
    msg.reply_ping(&ctx.http, "Server is already awake").await?;
    return Ok(());
  }

  match wol.awake() {
    Ok(_) => {
      msg.reply_ping(&ctx.http, "Server is waking").await?;
    }
    Err(e) => {
      Debug::inst("server_wake").log(&format!("Failed to wake Game Server - {}", e));
      msg
        .reply_ping(&ctx.http, "Couldn't wake the server :(")
        .await?;
      return Ok(());
    }
  }

  Debug::inst("server_wake").log("Server has Woken");
  Ok(())
}
