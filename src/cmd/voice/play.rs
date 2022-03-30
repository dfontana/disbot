use std::{collections::HashMap, error::Error};

use crate::{cmd::voice::connect_util::ChannelDisconnectBuilder, emoji::EmojiLookup};
use serenity::{
  async_trait,
  client::Context,
  model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandInteractionDataOption,
    ApplicationCommandInteractionDataOptionValue,
  },
  utils::MessageBuilder,
};

use tracing::error;

use songbird::{
  driver::Bitrate,
  input::{restartable::Restartable, Input},
};

use super::connect_util::ChannelDisconnect;
use super::SubCommandHandler;

#[derive(Default)]
pub struct Play {}

#[async_trait]
impl SubCommandHandler for Play {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &ApplicationCommandInteraction,
    subopt: &ApplicationCommandInteractionDataOption,
  ) -> Result<(), Box<dyn Error>> {
    // 1 arg: link. String.
    let chan = ChannelDisconnect::get_chan();
    let _ = chan.send(true).await;

    let guild_id = match itx.guild_id {
      Some(g) => g,
      None => {
        return Err("No Guild Id on Interaction".into());
      }
    };

    let args: HashMap<String, _> = subopt
      .options
      .iter()
      .map(|d| (d.name.to_owned(), d.resolved.to_owned()))
      .collect();

    let maybe_args = args
      .get("link")
      .map(|v| v.to_owned())
      .flatten()
      .and_then(|d| match d {
        ApplicationCommandInteractionDataOptionValue::String(v) => Some(v),
        _ => None,
      })
      .ok_or("Must provide a url|search string");
    let searchterm = match maybe_args {
      Ok(v) => v.trim().to_string(),
      Err(e) => {
        itx
          .create_followup_message(&ctx.http, |f| f.content(&format!("{:?}", e)))
          .await?;
        return Ok(());
      }
    };

    // Lookup context necessary to connect
    let channel_id = ctx
      .cache
      .guild(guild_id)
      .await
      .unwrap()
      .voice_states
      .get(&itx.user.id)
      .and_then(|voice_state| voice_state.channel_id);
    let connect_to = match channel_id {
      Some(channel) => channel,
      None => {
        itx
          .create_followup_message(&ctx.http, |f| f.content("Not in a voice channel"))
          .await?;
        return Ok(());
      }
    };

    // Fetch the Songbird mgr & join channel
    let manager = songbird::get(ctx)
      .await
      .expect("Songbird Voice client placed in at initialisation.")
      .clone();
    let (handler_lock, _success) = manager.join(guild_id, connect_to).await;

    // Add disconnect handler as needed
    let _reg = ChannelDisconnectBuilder::default()
      .manager(manager)
      .http(ctx.http.clone())
      .guild(guild_id)
      .channel(itx.channel_id)
      .emoji(EmojiLookup::inst().get(guild_id, &ctx.cache).await?)
      .build()?
      .maybe_register_handler(&handler_lock)
      .await;

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
        itx
          .create_followup_message(&ctx.http, |f| f.content("Error sourcing ffmpeg"))
          .await?;
        return Ok(());
      }
    };

    let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;

    let metadata = input.metadata.clone();
    let title = metadata
      .track
      .or(metadata.title)
      .unwrap_or_else(|| "<UNKNOWN>".to_string())
      .to_string();
    let source_url = metadata
      .source_url
      .unwrap_or_else(|| "Unknown Source".to_string())
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
    itx
      .create_followup_message(&ctx.http, |f| f.content(build.build()))
      .await?;

    let _ = chan.send(false).await;
    Ok(())
  }
}
