use serenity::{
  client::Context,
  framework::standard::{
    macros::{command, group},
    Args, CommandResult,
  },
  model::channel::Message,
  Result as SerenityResult,
};

use songbird::driver::Bitrate;
use songbird::input::restartable::Restartable;

#[group]
#[description = "Stream sound to the channel"]
#[summary = "Sheebs Givith Loud Noises"]
#[prefix = "p"]
#[only_in(guilds)]
#[default_command(play)]
#[commands(skip, stop)]
pub struct Voice;

#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
  // TODO clean this up
  // TODO trackqueue https://serenity-rs.github.io/songbird/current/songbird/tracks/struct.TrackQueue.html
  // TODO form driver https://serenity-rs.github.io/songbird/current/songbird/struct.Call.html
  // TODO disconnect the bot with the stop command or something
  let guild = msg.guild(&ctx.cache).await.unwrap();
  let guild_id = guild.id;

  let channel_id = guild
    .voice_states
    .get(&msg.author.id)
    .and_then(|voice_state| voice_state.channel_id);

  let connect_to = match channel_id {
    Some(channel) => channel,
    None => {
      println!("Not in channel");
      check_msg(msg.reply(ctx, "Not in a voice channel").await);
      return Ok(());
    }
  };

  let manager = songbird::get(ctx)
    .await
    .expect("Songbird Voice client placed in at initialisation.")
    .clone();

  let _handler = manager.join(guild_id, connect_to).await;

  let url = match args.single::<String>() {
    Ok(url) => url,
    Err(_) => {
      check_msg(
        msg
          .channel_id
          .say(&ctx.http, "Must provide a URL to a video or audio")
          .await,
      );

      return Ok(());
    }
  };

  if !url.starts_with("http") {
    check_msg(
      msg
        .channel_id
        .say(&ctx.http, "Must provide a valid URL")
        .await,
    );

    return Ok(());
  }

  let guild = msg.guild(&ctx.cache).await.unwrap();
  let guild_id = guild.id;

  let manager = songbird::get(ctx)
    .await
    .expect("Songbird Voice client placed in at initialisation.")
    .clone();

  if let Some(handler_lock) = manager.get(guild_id) {
    let mut handler = handler_lock.lock().await;
    handler.set_bitrate(Bitrate::Max);

    // Here, we use lazy restartable sources to make sure that we don't pay
    // for decoding, playback on tracks which aren't actually live yet.
    let source = match Restartable::ytdl(url, true).await {
      Ok(source) => source,
      Err(why) => {
        println!("Err starting source: {:?}", why);

        check_msg(msg.channel_id.say(&ctx.http, "Error sourcing ffmpeg").await);

        return Ok(());
      }
    };

    handler.enqueue_source(source.into());

    check_msg(
      msg
        .channel_id
        .say(
          &ctx.http,
          format!("Added song to queue: position {}", handler.queue().len()),
        )
        .await,
    );
  } else {
    check_msg(
      msg
        .channel_id
        .say(&ctx.http, "Not in a voice channel to play in")
        .await,
    );
  }

  Ok(())
}

#[command]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
  let guild = msg.guild(&ctx.cache).await.unwrap();
  let guild_id = guild.id;

  let manager = songbird::get(ctx)
    .await
    .expect("Songbird Voice client placed in at initialisation.")
    .clone();

  if let Some(handler_lock) = manager.get(guild_id) {
    let handler = handler_lock.lock().await;
    let queue = handler.queue();
    let _ = queue.skip();

    check_msg(
      msg
        .channel_id
        .say(
          &ctx.http,
          format!("Song skipped: {} in queue.", queue.len()),
        )
        .await,
    );
  } else {
    check_msg(
      msg
        .channel_id
        .say(&ctx.http, "Not in a voice channel to play in")
        .await,
    );
  }

  Ok(())
}

#[command]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
  let guild = msg.guild(&ctx.cache).await.unwrap();
  let guild_id = guild.id;

  let manager = songbird::get(ctx)
    .await
    .expect("Songbird Voice client placed in at initialisation.")
    .clone();

  if let Some(handler_lock) = manager.get(guild_id) {
    let handler = handler_lock.lock().await;
    let queue = handler.queue();
    let _ = queue.stop();

    check_msg(msg.channel_id.say(&ctx.http, "Queue cleared.").await);
  } else {
    check_msg(
      msg
        .channel_id
        .say(&ctx.http, "Not in a voice channel to play in")
        .await,
    );
  }

  Ok(())
}

fn check_msg(result: SerenityResult<Message>) {
  if let Err(why) = result {
    println!("Error sending message: {:?}", why);
  }
}
