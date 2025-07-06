use crate::config::Config;
use crate::emoji::EmojiLookup;
use anyhow::anyhow;
use derive_new::new;
use serenity::{
  async_trait,
  model::{channel::Message, channel::ReactionType, guild::Emoji},
  prelude::Context,
};
use tracing::{info, instrument};

use super::MessageListener;

#[derive(new)]
pub struct ShrugHandler {
  config: Config,
  emoji: EmojiLookup,
}

impl ShrugHandler {
  async fn react_and_send(
    &self,
    emoji: Emoji,
    ctx: &Context,
    msg: &Message,
  ) -> Result<(), anyhow::Error> {
    let react = msg.react(
      &ctx.http,
      ReactionType::Custom {
        animated: emoji.animated,
        id: emoji.id,
        name: Some(emoji.name.to_string()),
      },
    );
    let message = msg.channel_id.say(&ctx.http, format!("{}", emoji));
    tokio::try_join!(react, message)
      .map(|_| ())
      .map_err(|_| anyhow!("Failed to react/Send"))
  }
}

#[async_trait]
impl MessageListener for ShrugHandler {
  #[instrument(name = "Shrug", level = "INFO", skip(self, ctx, msg))]
  async fn message(&self, ctx: &Context, msg: &Message) -> Result<(), anyhow::Error> {
    if msg.author.id == ctx.cache.as_ref().current_user().id {
      info!("Skipping, self message");
      return Ok(());
    }

    let guild_id = match msg.guild_id {
      Some(id) => id,
      None => return Ok(()),
    };

    let mentions_user = msg.mentions.iter().find(|user| {
      self
        .config
        .emote_users
        .iter()
        .any(|cname| *cname.to_lowercase() == user.name.to_lowercase())
    });

    if mentions_user.is_none() {
      info!("Did not find a matching user mention");
      return Ok(());
    }

    let emoji = self.emoji.get(&ctx.http, guild_id).await?;
    self.react_and_send(emoji, ctx, msg).await
  }
}
