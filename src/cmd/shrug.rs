use crate::config::Config;
use crate::emoji::EmojiLookup;
use derive_new::new;
use serenity::{
  async_trait,
  model::{channel::Message, channel::ReactionType, guild::Emoji},
  prelude::Context,
};
use tracing::{error, info, instrument};

use super::MessageListener;

#[derive(new)]
pub struct ShrugHandler {
  config: Config,
  emoji: EmojiLookup,
}

impl ShrugHandler {
  async fn react_and_send(&self, emoji: Emoji, ctx: &Context, msg: &Message) -> Result<(), String> {
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
      .map_err(|_| "Failed to react/Send".to_string())
  }
}

#[async_trait]
impl MessageListener for ShrugHandler {
  #[instrument(name = "Shrug", level = "INFO", skip(self, ctx, msg))]
  async fn message(&self, ctx: &Context, msg: &Message) {
    if msg.is_own(&ctx.cache) {
      info!("Skipping, self message");
      return;
    }

    let guild_id = match msg.guild_id {
      Some(id) => id,
      None => return,
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
      return;
    }

    let emoji = self.emoji.get(&ctx.http, &ctx.cache, guild_id).await;

    let send = match emoji {
      Ok(e) => self.react_and_send(e, ctx, msg).await,
      Err(cause) => Err(cause),
    };

    match send {
      Ok(_) => {}
      Err(cause) => {
        error!("Failed to react {:?}", cause);
        if let Err(why) = msg
          .channel_id
          .say(&ctx.http, "You taketh my shrug, you taketh me :(")
          .await
        {
          error!("Failed to send error {:?}", why);
          return;
        }
      }
    }
  }
}
