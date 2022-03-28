use crate::emoji::EmojiLookup;
use serenity::{
  client::Context,
  // framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
  utils::MessageBuilder,
};
use tracing::instrument;

// #[command]
// #[description = "Move a the specified track to a specific position in queue. You cannot move the current track."]
// #[aliases(move)]
// #[usage = "move {posA} {posB}"]
// #[example = "move 3 2"]
// #[num_args(2)]
// #[only_in(guilds)]
// async fn reorder(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
//   exec_reorder(ctx, msg, args).await
// }

// #[instrument(name = "VoiceReorder", level = "info", skip(ctx, msg))]
// async fn exec_reorder(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
//   // Get the handler
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

//   //  Validate the position args
//   let queue_size = handler.queue().current_queue().len();
//   let posa = match validate_position(args.single::<usize>(), queue_size) {
//     Ok(v) => v,
//     Err(e) => {
//       let _ = msg.channel_id.say(&ctx.http, &e).await;
//       return Ok(());
//     }
//   };
//   let posb = match validate_position(args.single::<usize>(), queue_size) {
//     Ok(v) => v,
//     Err(e) => {
//       let _ = msg.channel_id.say(&ctx.http, &e).await;
//       return Ok(());
//     }
//   };
//   if posa == posb {
//     let _ = msg
//       .channel_id
//       .say(&ctx.http, "A touch psychotic are we?")
//       .await;
//     return Ok(());
//   }

//   // Perform the movement
//   handler.queue().modify_queue(|queue| {
//     if let Some(item) = queue.remove(posa - 1) {
//       // b/c queue is 0 based
//       if posb - 1 < queue.len() {
//         queue.insert(posb - 1, item);
//       } else {
//         queue.push_back(item);
//       }
//     }
//   });

//   let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;
//   let _ = msg
//     .channel_id
//     .say(
//       &ctx.http,
//       MessageBuilder::new()
//         .mention(&emoji)
//         .push_bold("Queued updated!")
//         .mention(&emoji)
//         .push_line("")
//         .push_italic("You can list the queue your damn self")
//         .build(),
//     )
//     .await;

//   Ok(())
// }

// fn validate_position<T>(maybe_pos: Result<usize, T>, queue_size: usize) -> Result<usize, String> {
//   let pos = match maybe_pos {
//     Err(_) => return Err("Must provide a numeric position".into()),
//     Ok(v) => v,
//   };
//   if pos <= 1 {
//     return Err("Cannot move first item".into());
//   }
//   if pos > queue_size {
//     return Err("Can only move item to end of queue".into());
//   }
//   Ok(pos)
// }
