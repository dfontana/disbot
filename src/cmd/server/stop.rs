use crate::cmd::server::wol::Wol;
use serenity::{
  client::Context,
  // framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
};
use tracing::{error, info, instrument};

// #[command]
// #[description = "Stop the game server"]
// #[usage = "stop"]
// #[example = "stop"]
// async fn stop(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
//   exec_stop(ctx, msg).await
// }

// #[instrument(name = "ServerStop", level = "INFO", skip(ctx, msg))]
// async fn exec_stop(ctx: &Context, msg: &Message) -> CommandResult {
//   let wol = Wol::inst()?;

//   let is_awake = match wol.is_awake() {
//     Ok(v) => v,
//     Err(e) => {
//       error!("Failed to check Game Server is awake - {:?}", e);
//       msg
//         .reply_ping(&ctx.http, "Couldn't stop the server :(")
//         .await?;
//       return Ok(());
//     }
//   };

//   if !is_awake {
//     msg.reply_ping(&ctx.http, "Server is not awake").await?;
//     return Ok(());
//   }

//   match wol.shutdown() {
//     Ok(0) => {
//       msg.reply_ping(&ctx.http, "Server is stopping").await?;
//     }
//     Ok(left) => {
//       let msg_res = format!(
//         "Stop ran recently, please wait {}m{}s",
//         left / 60,
//         left % 60
//       );
//       msg.reply_ping(&ctx.http, msg_res).await?;
//     }
//     Err(e) => {
//       error!("Failed to stop Game Server - {:?}", e);
//       msg
//         .reply_ping(&ctx.http, "Couldn't stop the server :(")
//         .await?;
//       return Ok(());
//     }
//   }

//   info!("Server has stopped");
//   Ok(())
// }
