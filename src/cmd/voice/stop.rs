use std::error::Error;

use super::SubCommandHandler;
use crate::{cmd::voice::connect_util::ChannelDisconnectBuilder, emoji::EmojiLookup};
use serenity::{
  async_trait,
  client::Context,
  model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandInteractionDataOption,
  },
};
use tracing::info;

#[derive(Default)]
pub struct Stop {}

#[async_trait]
impl SubCommandHandler for Stop {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &ApplicationCommandInteraction,
    _subopt: &ApplicationCommandInteractionDataOption,
  ) -> Result<(), Box<dyn Error>> {
    let guild_id = match itx.guild_id {
      Some(g) => g,
      None => {
        return Err("No Guild Id on Interaction".into());
      }
    };

    info!("Stopping voice playback");
    let _stop = ChannelDisconnectBuilder::default()
      .manager(
        songbird::get(ctx)
          .await
          .expect("Songbird Voice client placed in at initialisation.")
          .clone(),
      )
      .http(ctx.http.clone())
      .guild(guild_id)
      .channel(itx.channel_id)
      .emoji(EmojiLookup::inst().get(guild_id, &ctx.cache).await?)
      .build()?
      .stop()
      .await;

    Ok(())
  }
}
