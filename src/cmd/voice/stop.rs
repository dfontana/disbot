use std::error::Error;

use super::SubCommandHandler;
use crate::{cmd::voice::connect_util::ChannelDisconnectBuilder, emoji::EmojiLookup};
use derive_new::new;
use serenity::{
  async_trait,
  client::Context,
  model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption,
  },
};
use tracing::info;

#[derive(new)]
pub struct Stop {
  emoji: EmojiLookup,
}

#[async_trait]
impl SubCommandHandler for Stop {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &ApplicationCommandInteraction,
    _subopt: &CommandDataOption,
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
      .emoji(self.emoji.get(&ctx.http, &ctx.cache, guild_id).await?)
      .build()?
      .stop()
      .await;

    Ok(())
  }
}
