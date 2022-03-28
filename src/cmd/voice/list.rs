use serenity::{
  client::Context,
  // framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
  utils::MessageBuilder,
};
use tracing::instrument;

// #[command]
// #[description = "Show what's currently queued"]
// #[only_in(guilds)]
// async fn list(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
//   exec_list(ctx, msg).await
// }

// #[instrument(name = "VoiceList", level = "INFO", skip(ctx, msg))]
// async fn exec_list(ctx: &Context, msg: &Message) -> CommandResult {
//   let guild = msg.guild(&ctx.cache).await.unwrap();
//   let guild_id = guild.id;

//   let manager = songbird::get(ctx)
//     .await
//     .expect("Songbird Voice client placed in at initialisation.")
//     .clone();
//   let handler_lock = match manager.get(guild_id) {
//     None => {
//       let _ = msg
//         .channel_id
//         .say(&ctx.http, "I'm currently not in a voice channel")
//         .await;
//       return Ok(());
//     }
//     Some(v) => v,
//   };
//   let handler = handler_lock.lock().await;

//   let mut bld = MessageBuilder::new();
//   bld.push_bold_line("Current Queue:");
//   let mut body = String::new();
//   for (idx, trk) in handler.queue().current_queue().iter().enumerate() {
//     body.push_str(&format!(
//       "{}. '{}'\n",
//       idx + 1,
//       trk
//         .metadata()
//         .track
//         .as_ref()
//         .or_else(|| trk.metadata().title.as_ref())
//         .unwrap_or(&"<UNKNOWN>".to_string())
//     ));
//   }

//   let _ = msg
//     .channel_id
//     .say(&ctx.http, bld.push_codeblock(body, None).build())
//     .await;
//   Ok(())
// }
