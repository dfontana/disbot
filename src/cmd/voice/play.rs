use std::{collections::HashMap, error::Error};

use crate::{
  actor::ActorHandle,
  cmd::voice::connect_util::{DisconnectDetails, DisconnectEventHandler},
  config::Config,
  emoji::EmojiLookup,
};
use derive_new::new;
use serenity::{
  all::{CommandDataOption, CommandInteraction, ResolvedValue},
  async_trait,
  builder::EditInteractionResponse,
  client::Context,
  utils::MessageBuilder,
};

use tracing::{error, info};

use super::{connect_util::DisconnectMessage, SubCommandHandler};
use songbird::{driver::Bitrate, input::Input};

#[derive(new)]
pub struct Play {
  config: Config,
  emoji: EmojiLookup,
  disconnect: ActorHandle<DisconnectMessage>,
}

#[async_trait]
impl SubCommandHandler for Play {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
    subopt: &CommandDataOption,
  ) -> Result<(), Box<dyn Error>> {
    // 1 arg: link. String.
    let _ = self.disconnect.send(DisconnectMessage::Enqueue).await;
    let res = wrapped_handle(self, ctx, itx, subopt).await;
    let _ = self.disconnect.send(DisconnectMessage::Dequeue).await;
    match res {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }
}

async fn wrapped_handle(
  play: &Play,
  ctx: &Context,
  itx: &CommandInteraction,
  subopt: &CommandDataOption,
) -> Result<(), Box<dyn Error + Send + Sync>> {
  let guild_id = match itx.guild_id {
    Some(g) => g,
    None => {
      return Err("No Guild Id on Interaction".into());
    }
  };

  let args: HashMap<String, _> = itx
    .data
    .options()
    .iter()
    .map(|d| (d.name.to_owned(), d.value.to_owned()))
    .collect();

  let maybe_args = args
    .get("link_or_search")
    .and_then(|d| match d {
      ResolvedValue::String(v) => Some(v),
      _ => None,
    })
    .ok_or("Must provide a url|search string");
  let searchterm = match maybe_args {
    Ok(v) => v.trim().to_string(),
    Err(e) => {
      itx
        .edit_response(
          &ctx.http,
          EditInteractionResponse::new().content(&format!("{:?}", e)),
        )
        .await?;
      return Ok(());
    }
  };

  // Lookup context necessary to connect
  let channel_id = ctx
    .cache
    .guild(guild_id)
    .unwrap()
    .voice_states
    .get(&itx.user.id)
    .and_then(|voice_state| voice_state.channel_id);
  let connect_to = match channel_id {
    Some(channel) => channel,
    None => {
      itx
        .edit_response(
          &ctx.http,
          EditInteractionResponse::new().content("Not in a voice channel"),
        )
        .await?;
      return Ok(());
    }
  };

  // Fetch the Songbird mgr & join channel
  let manager = songbird::get(ctx)
    .await
    .expect("Songbird Voice client placed in at initialisation.")
    .clone();

  // Check if we're already in the channel or not, connecting if not
  let handler_lock = match manager.get(guild_id) {
    None => {
      info!("Joining voice for first time...");
      let handler_lock = match manager.join(guild_id, connect_to).await {
        Ok(v) => v,
        Err(why) => {
          error!("Err joining voice: {:?}", why);
          itx
            .edit_response(
              &ctx.http,
              EditInteractionResponse::new().content("Error joining voice channel"),
            )
            .await?;
          return Ok(());
        }
      };

      // Register an event handler to listen for the duration of the call
      DisconnectEventHandler::register(play.config.timeout, play.disconnect.clone(), &handler_lock)
        .await;

      // Inform disconnect of where to disconnect from
      play
        .disconnect
        .send(DisconnectMessage::Details(DisconnectDetails::new(
          handler_lock.clone(),
          ctx.http.clone(),
          play.emoji.get(&ctx.http, &ctx.cache, guild_id).await?,
        )))
        .await;

      handler_lock
    }
    Some(l) => {
      {
        // Rejoin the channel if we're not in it already, but we previously were
        let mut lock = l.lock().await;
        if lock.current_channel().is_none() {
          let _ = lock.join(connect_to).await;
        }
      }
      l
    }
  };

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
        .edit_response(
          &ctx.http,
          EditInteractionResponse::new().content("Error sourcing ffmpeg"),
        )
        .await?;
      return Ok(());
    }
  };

  let emoji = play.emoji.get(&ctx.http, &ctx.cache, guild_id).await?;

  let metadata = input.aux_metadata().await;
  let title = match metadata.map(|m| m.track.or(m.title)) {
    Ok(Some(v)) => v,
    Err(_) | Ok(None) => "<UNKNOWN>".to_string(),
  };
  let source_url = match metadata.map(|m| m.source_url) {
    Ok(Some(v)) => v,
    Err(_) | Ok(None) => "Unknown Source".to_string(),
  };
  let mut handler = handler_lock.lock().await;
  handler.set_bitrate(Bitrate::Max);
  handler.enqueue_source(input);

  let mut build = MessageBuilder::new();
  build
    .push_bold("Queued")
    .push(format!(" ({}) ", handler.queue().len()))
    .push_mono(title)
    .emoji(&emoji);
  if !is_url {
    build.push_line("").push(source_url);
  }
  itx
    .edit_response(
      &ctx.http,
      EditInteractionResponse::new().content(build.build()),
    )
    .await?;

  Ok(())
}
