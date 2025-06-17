use super::{connect_util::DisconnectMessage, SubCommandHandler};
use crate::{
  actor::ActorHandle,
  cmd::{
    arg_util::Args,
    voice::connect_util::{DisconnectDetails, DisconnectEventHandler},
  },
  config::Config,
  emoji::EmojiLookup,
  HttpClient,
};
use anyhow::anyhow;
use derive_new::new;
use serenity::{
  all::CommandInteraction, async_trait, builder::EditInteractionResponse, client::Context,
  utils::MessageBuilder,
};
use songbird::{
  driver::Bitrate,
  input::{Input, YoutubeDl},
  tracks::Track,
};
use std::sync::Arc;
use tracing::info;

#[derive(Clone, Debug)]
pub struct ListMetadata {
  pub title: String,
  pub url: String,
}

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
    args: &Args,
  ) -> Result<(), anyhow::Error> {
    let _ = self.disconnect.send(DisconnectMessage::Enqueue).await;
    let res = wrapped_handle(self, ctx, itx, args).await;
    let _ = self.disconnect.send(DisconnectMessage::Dequeue).await;
    res
  }
}

async fn wrapped_handle(
  play: &Play,
  ctx: &Context,
  itx: &CommandInteraction,
  args: &Args<'_>,
) -> Result<(), anyhow::Error> {
  let (guild_id, channel_id) = {
    let guild = itx
      .guild_id
      .ok_or_else(|| anyhow!("No Guild Id on Interaction"))?;
    let channel = ctx
      .cache
      .guild(guild)
      .and_then(|g| {
        g.voice_states
          .get(&itx.user.id)
          .and_then(|vs| vs.channel_id)
      })
      .ok_or_else(|| anyhow!("Not in a voice channel"))?;
    (guild, channel)
  };

  // 1 arg: link. String.
  let searchterm = args
    .str("link_or_search")
    .map_err(|e| anyhow!("Must provide a url|search string").context(e))?
    .to_string();

  // Fetch the Songbird mgr & join channel
  let manager = songbird::get(ctx)
    .await
    .expect("Songbird Voice client placed in at initialisation.")
    .clone();

  // Check if we're already in the channel or not, connecting if not
  let handler_lock = match manager.get(guild_id) {
    None => {
      info!("Joining voice for first time...");
      let handler_lock = manager
        .join(guild_id, channel_id)
        .await
        .map_err(|e| anyhow!("Error joining voice channel").context(format!("{:?}", e)))?;

      // Register an event handler to listen for the duration of the call
      DisconnectEventHandler::register(
        play.config.voice_channel_timeout_seconds,
        play.disconnect.clone(),
        &handler_lock,
      )
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
          let _ = lock.join(channel_id).await;
        }
      }
      l
    }
  };

  // Queue up the source
  let http_client = {
    let data = ctx.data.read().await;
    data
      .get::<HttpClient>()
      .cloned()
      .expect("Guaranteed to exist in the typemap.")
  };
  let is_url = searchterm.starts_with("http");
  let resolved_src = match is_url {
    false => YoutubeDl::new_search(http_client, searchterm),
    true => YoutubeDl::new(http_client, searchterm),
  };
  let mut input = Input::from(resolved_src);

  let (title, source_url) = input
    .aux_metadata()
    .await
    .map(|m| {
      let title = m.track.or(m.title);
      let url = m.source_url;
      (title, url)
    })
    .unwrap_or_default();
  let list_metadata = ListMetadata {
    title: title.unwrap_or_else(|| "<UNKNOWN>".to_string()),
    url: source_url.unwrap_or_else(|| "<UNKNOWN>".to_string()),
  };

  let mut handler = handler_lock.lock().await;
  handler.set_bitrate(Bitrate::Max);

  // Create track with custom metadata using songbird 0.5.0 API
  let track_with_data = Track::new_with_data(
    input,
    Arc::new(list_metadata.clone()) as Arc<dyn std::any::Any + Send + Sync>,
  );
  let _th = handler.enqueue(track_with_data).await;

  let emoji = play.emoji.get(&ctx.http, &ctx.cache, guild_id).await?;
  let mut build = MessageBuilder::new();
  build
    .push_bold("Queued")
    .push(format!(" ({}) ", handler.queue().len()))
    .push_mono(list_metadata.title)
    .emoji(&emoji);
  if !is_url {
    build.push_line("").push(list_metadata.url);
  }
  itx
    .edit_response(
      &ctx.http,
      EditInteractionResponse::new().content(build.build()),
    )
    .await?;

  Ok(())
}
