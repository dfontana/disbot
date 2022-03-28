use std::{thread, time::Duration};

use crate::cmd::server::wol::Wol;
use serenity::{
  client::Context,
  // framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
};
use tracing::{error, info, instrument};

// #[command]
// #[description = "Start the game server"]
// #[usage = "start"]
// #[example = "start"]
// async fn start(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
//   exec_start(ctx, msg).await
// }

// #[instrument(name = "ServerStart", level = "INFO", skip(ctx, msg))]
// async fn exec_start(ctx: &Context, msg: &Message) -> CommandResult {
//   let wol = Wol::inst()?;

//   let is_awake = match wol.is_awake() {
//     Ok(v) => v,
//     Err(e) => {
//       error!("Failed to check Game Server is awake - {}", e);
//       msg
//         .reply_ping(&ctx.http, "Couldn't start the server :(")
//         .await?;
//       return Ok(());
//     }
//   };

//   if is_awake {
//     msg.reply_ping(&ctx.http, "Server is already awake").await?;
//     return Ok(());
//   }

//   match wol.awake() {
//     Ok(_) => {
//       msg
//         .reply_ping(
//           &ctx.http,
//           "Server is waking... (I'll let you known when its up)",
//         )
//         .await?;
//     }
//     Err(e) => {
//       error!("Failed to start Game Server - {:?}", e);
//       msg
//         .reply_ping(&ctx.http, "Couldn't start the server :(")
//         .await?;
//       return Ok(());
//     }
//   }

//   let mut keep_trying = 12;
//   while keep_trying > 0 {
//     thread::sleep(Duration::from_secs(10));
//     match wol.is_awake() {
//       Ok(v) => {
//         keep_trying -= 1;
//         if v {
//           keep_trying = 0;
//           msg.reply_ping(&ctx.http, "Server is awake!").await?;
//         }
//       }
//       Err(e) => {
//         keep_trying = 0;
//         error!("Failed to check if Game Server is live - {:?}", e);
//         msg
//           .reply_ping(&ctx.http, "Failed to check Game Server is awake")
//           .await?;
//       }
//     }
//   }

//   info!("Server has Woken");
//   Ok(())
// }
