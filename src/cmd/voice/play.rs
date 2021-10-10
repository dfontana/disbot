use crate::emoji::EmojiLookup;
use serenity::{
  async_trait,
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::{channel::Message, id::GuildId},
  prelude::RwLock,
  utils::MessageBuilder,
  FutureExt,
};
use std::{sync::Arc, time::Duration};
use tracing::{error, info, instrument};

use songbird::{
  driver::Bitrate,
  input::{restartable::Restartable, Input},
  Event, EventContext, EventHandler, Songbird,
};

lazy_static! {
  static ref HANDLER_ADDED: RwLock<bool> = RwLock::new(false);
}

#[command]
#[description = "Play a sound clip via link or search term"]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
  exec_play(ctx, msg, args).await
}

#[instrument(name = "VoicePlay", level = "INFO", skip(ctx, msg, args))]
async fn exec_play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
  let maybe_args = match args.len() {
    0 => Err("Must provide a url|search string"),
    1 => args
      .single::<String>()
      .map_err(|_| "Must provide a url|search string"),
    _ => Ok(args.iter::<String>().fold(String::new(), |mut a, b| {
      a.push_str(" ");
      a.push_str(&b.unwrap());
      a
    })),
  };
  let searchterm = match maybe_args {
    Ok(v) => v.trim().to_string(),
    Err(e) => {
      let _rep = msg.reply(ctx, &format!("{:?}", e)).await;
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
  let (handler_lock, _success) = manager.join(guild_id, connect_to).await;

  // On first connect, add disconnect handler
  if !HANDLER_ADDED.read().map(|g| *g).await {
    let _fut = HANDLER_ADDED.write().map(|mut g| *g = true).await;
    let mut handler = handler_lock.lock().await;
    handler.add_global_event(
      Event::Periodic(Duration::from_secs(30), None),
      ChannelDisconnect {
        manager,
        guild: guild_id,
      },
    );
  }

  // Queue up the source
  let is_url = searchterm.starts_with("http");
  let resolved_src = match is_url {
    true => Restartable::ytdl(searchterm, true).await,
    false => Restartable::ytdl_search(searchterm, true).await,
  };

  let input = match resolved_src {
    Ok(inp) => Input::from(inp),
    Err(why) => {
      error!("Err starting source: {:?}", why);
      let _ = msg.channel_id.say(&ctx.http, "Error sourcing ffmpeg").await;
      return Ok(());
    }
  };

  let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;

  let metadata = input.metadata.clone();
  let title = metadata
    .track
    .or(metadata.title)
    .unwrap_or("<UNKNOWN>".to_string())
    .to_string();
  let source_url = metadata
    .source_url
    .unwrap_or("Unknown Source".to_string())
    .to_string();

  let mut handler = handler_lock.lock().await;
  handler.set_bitrate(Bitrate::Max);
  handler.enqueue_source(input);

  let mut build = MessageBuilder::new();
  build
    .push_bold("Queued")
    .push(format!(" ({}) ", handler.queue().len()))
    .push_mono(title)
    .mention(&emoji);
  if !is_url {
    build.push_line("").push(source_url);
  }
  let _ = msg.channel_id.say(&ctx.http, build.build()).await;
  Ok(())
}

struct ChannelDisconnect {
  manager: Arc<Songbird>,
  guild: GuildId,
}

#[async_trait]
impl EventHandler for ChannelDisconnect {
  #[instrument(name = "VoiceTimeoutListener", level = "INFO", skip(self, _ctx))]
  async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
    info!("Checking for inactivity...");
    let should_close = match self.manager.get(self.guild) {
      None => false,
      Some(handler_lock) => handler_lock.lock().await.queue().is_empty(),
    };
    if should_close {
      info!("Disconnecting client for inactivity");
      let _dc = self.manager.leave(self.guild).await;
      info!("Disconnected");
    }
    None
  }
}
