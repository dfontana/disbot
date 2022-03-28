use crate::emoji::EmojiLookup;

use serenity::{
  client::Context,
  // framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
  utils::MessageBuilder,
};
use tracing::instrument;

// #[command]
// #[description = "Skip the currently playing sound"]
// #[only_in(guilds)]
// async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
//   exec_skip(ctx, msg).await
// }

// #[instrument(name = "VoiceSkip", level = "INFO", skip(ctx, msg))]
// async fn exec_skip(ctx: &Context, msg: &Message) -> CommandResult {
//   let guild = msg.guild(&ctx.cache).await.unwrap();
//   let guild_id = guild.id;

//   let manager = songbird::get(ctx)
//     .await
//     .expect("Songbird Voice client placed in at initialisation.")
//     .clone();

//   let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;

//   match manager.get(guild_id) {
//     None => {
//       let _ = msg
//         .channel_id
//         .say(&ctx.http, "Not in a voice channel to play in")
//         .await;
//     }
//     Some(handler_lock) => {
//       let handler = handler_lock.lock().await;
//       let queue = handler.queue();
//       let _ = queue.skip();
//       let _res = msg
//         .channel_id
//         .say(
//           &ctx.http,
//           MessageBuilder::new()
//             .push("I didn't like that song either ")
//             .mention(&emoji)
//             .build(),
//         )
//         .await;
//     }
//   }
//   Ok(())
// }
