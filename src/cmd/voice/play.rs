use crate::{cmd::voice::connect_util::ChannelDisconnectBuilder, emoji::EmojiLookup};
use serenity::{
  client::Context,
  // framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
  utils::MessageBuilder,
};

use tracing::{error, instrument};

use songbird::{
  driver::Bitrate,
  input::{restartable::Restartable, Input},
};

use super::connect_util::ChannelDisconnect;

// #[command]
// #[description = "Play a sound clip via link or search term"]
// #[only_in(guilds)]
// async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
//   let chan = ChannelDisconnect::get_chan();
//   let _ = chan.send(true).await;
//   let res = exec_play(ctx, msg, args).await;
//   let _ = chan.send(false).await;
//   res
// }

// #[instrument(name = "VoicePlay", level = "INFO", skip(ctx, msg, args))]
// async fn exec_play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
//   let maybe_args = match args.len() {
//     0 => Err("Must provide a url|search string"),
//     1 => args
//       .single::<String>()
//       .map_err(|_| "Must provide a url|search string"),
//     _ => Ok(args.iter::<String>().fold(String::new(), |mut a, b| {
//       a.push(' ');
//       a.push_str(&b.unwrap());
//       a
//     })),
//   };
//   let searchterm = match maybe_args {
//     Ok(v) => v.trim().to_string(),
//     Err(e) => {
//       let _rep = msg.reply(ctx, &format!("{:?}", e)).await;
//       return Ok(());
//     }
//   };

//   // Lookup context necessary to connect
//   let guild = msg.guild(&ctx.cache).await.unwrap();
//   let guild_id = guild.id;
//   let channel_id = guild
//     .voice_states
//     .get(&msg.author.id)
//     .and_then(|voice_state| voice_state.channel_id);
//   let connect_to = match channel_id {
//     Some(channel) => channel,
//     None => {
//       let _ = msg.reply(ctx, "Not in a voice channel").await;
//       return Ok(());
//     }
//   };

//   // Fetch the Songbird mgr & join channel
//   let manager = songbird::get(ctx)
//     .await
//     .expect("Songbird Voice client placed in at initialisation.")
//     .clone();
//   let (handler_lock, _success) = manager.join(guild_id, connect_to).await;

//   // Add disconnect handler as needed
//   let _reg = ChannelDisconnectBuilder::default()
//     .manager(manager)
//     .http(ctx.http.clone())
//     .guild(guild_id)
//     .channel(msg.channel_id)
//     .emoji(EmojiLookup::inst().get(guild_id, &ctx.cache).await?)
//     .build()?
//     .maybe_register_handler(&handler_lock)
//     .await;

//   // Queue up the source
//   let is_url = searchterm.starts_with("http");
//   let resolved_src = match is_url {
//     true => Restartable::ytdl(searchterm, true).await,
//     false => Restartable::ytdl_search(searchterm, true).await,
//   };

//   let input = match resolved_src {
//     Ok(inp) => Input::from(inp),
//     Err(why) => {
//       error!("Err starting source: {:?}", why);
//       let _ = msg.channel_id.say(&ctx.http, "Error sourcing ffmpeg").await;
//       return Ok(());
//     }
//   };

//   let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;

//   let metadata = input.metadata.clone();
//   let title = metadata
//     .track
//     .or(metadata.title)
//     .unwrap_or_else(|| "<UNKNOWN>".to_string())
//     .to_string();
//   let source_url = metadata
//     .source_url
//     .unwrap_or_else(|| "Unknown Source".to_string())
//     .to_string();

//   let mut handler = handler_lock.lock().await;
//   handler.set_bitrate(Bitrate::Max);
//   handler.enqueue_source(input);

//   let mut build = MessageBuilder::new();
//   build
//     .push_bold("Queued")
//     .push(format!(" ({}) ", handler.queue().len()))
//     .push_mono(title)
//     .mention(&emoji);
//   if !is_url {
//     build.push_line("").push(source_url);
//   }
//   let _ = msg.channel_id.say(&ctx.http, build.build()).await;
//   Ok(())
// }
