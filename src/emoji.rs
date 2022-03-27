use crate::config::Config;
use once_cell::sync::Lazy;
use serenity::{
  cache::Cache,
  model::{guild::Emoji, id::GuildId},
};

use std::sync::RwLock;

static INSTANCE: Lazy<RwLock<String>> = Lazy::new(|| RwLock::new("".to_string()));

pub struct EmojiLookup {}

pub fn configure(config: &Config) -> Result<(), String> {
  let mut inst = INSTANCE
    .try_write()
    .map_err(|_| "Failed to get lock on emoji instance")?;
  *inst = config.emote_name.to_string();
  Ok(())
}

impl EmojiLookup {
  pub fn inst() -> Self {
    EmojiLookup {}
  }
  pub async fn get(&self, guild_id: GuildId, cache: &Cache) -> Result<Emoji, String> {
    // Pull the emoji from the guild attached to the message
    let maybe_emoji = cache
      .guild_field(guild_id, |guild| guild.emojis.clone())
      .await
      .ok_or_else(|| "Failed to pull emojis for Guild".to_string())?;

    // If we do, though, we should find the emoji from the config
    let emoji = match INSTANCE.try_read() {
      Ok(e) => maybe_emoji
        .iter()
        .find_map(
          |(_, emoji)| {
            if emoji.name == *e {
              Some(emoji)
            } else {
              None
            }
          },
        )
        .ok_or_else(|| "Server does not have expected Emoji".to_string())?,
      Err(_) => return Err("Failed to get read on Emoji".to_string()),
    };

    Ok(emoji.clone())
  }
}
