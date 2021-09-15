use serenity::{
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
};

use songbird::{
  driver::Bitrate,
  input::{restartable::Restartable, Input},
};

#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
  // Pull out the URL requested if there is one
  let maybe_url = args
    .single::<String>()
    .map_err(|_| "Must provide a URL")
    .and_then(|url| match url.starts_with("http") {
      true => Ok(url),
      false => Err("Must send a valid URL"),
    });
  let url = match maybe_url {
    Ok(v) => v,
    Err(s) => {
      let _ = msg.reply(ctx, s).await;
      return Ok(());
    }
  };

  // Lookup context necessary to connect
  let guild = msg.guild(&ctx.cache).await.unwrap();
  let guild_id = guild.id;
  let channel_id = guild
    .voice_states
    .get(&msg.author.id)
    .and_then(|voice_state| voice_state.channel_id);
  let connect_to = match channel_id {
    Some(channel) => channel,
    None => {
      let _ = msg.reply(ctx, "Not in a voice channel").await;
      return Ok(());
    }
  };

  // Fetch the Songbird mgr & join channel
  let manager = songbird::get(ctx)
    .await
    .expect("Songbird Voice client placed in at initialisation.")
    .clone();
  let (handler_lock, success) = manager.join(guild_id, connect_to).await;

  // Queue up the source
  let source = match Restartable::ytdl(url, true).await {
    Ok(source) => source,
    Err(why) => {
      println!("Err starting source: {:?}", why);
      let _ = msg.channel_id.say(&ctx.http, "Error sourcing ffmpeg").await;
      return Ok(());
    }
  };
  let input = Input::from(source);

  let title = input
    .metadata
    .track
    .as_ref()
    .unwrap_or(&"<UNKNOWN>".to_string())
    .to_string();

  let mut handler = handler_lock.lock().await;
  handler.set_bitrate(Bitrate::Max);
  handler.enqueue_source(input);
  let reply = format!("**Queued** ({:?}) `{}`", handler.queue().len(), title,);
  let _ = msg.channel_id.say(&ctx.http, reply).await;
  Ok(())
}
