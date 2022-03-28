use crate::{cmd::voice::connect_util::ChannelDisconnectBuilder, emoji::EmojiLookup};
use serenity::{
  client::Context,
  // framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
};
use tracing::{info, instrument};

// #[command]
// #[description = "Stop all sound immediately & disconnect"]
// #[only_in(guilds)]
// async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
//   exec_stop(ctx, msg).await
// }

// #[instrument(name = "VoiceStop", level = "info", skip(ctx, msg))]
// async fn exec_stop(ctx: &Context, msg: &Message) -> CommandResult {
//   let guild_id = msg.guild(&ctx.cache).await.unwrap().id;

//   info!("Stopping voice playback");
//   let _stop = ChannelDisconnectBuilder::default()
//     .manager(
//       songbird::get(ctx)
//         .await
//         .expect("Songbird Voice client placed in at initialisation.")
//         .clone(),
//     )
//     .http(ctx.http.clone())
//     .guild(guild_id)
//     .channel(msg.channel_id)
//     .emoji(EmojiLookup::inst().get(guild_id, &ctx.cache).await?)
//     .build()?
//     .stop()
//     .await;

//   Ok(())
// }
