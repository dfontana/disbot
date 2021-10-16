use crate::config::Config;
use lazy_static::lazy_static;
use serenity::{
  cache::Cache,
  model::{channel::ReactionType, guild::Emoji, id::GuildId},
};

use std::{collections::HashMap, sync::RwLock};

lazy_static! {
  static ref INSTANCE: RwLock<String> = RwLock::new("".to_string());
  static ref NUMBERS: HashMap<usize, String> = {
    let mut m = HashMap::new();
    m.insert(0,  "\u{30}\u{fe0f}\u{20e3}".to_string());
    m.insert(1,  "\u{31}\u{fe0f}\u{20e3}".to_string());
    m.insert(2,  "\u{32}\u{fe0f}\u{20e3}".to_string());
    m.insert(3,  "\u{33}\u{fe0f}\u{20e3}".to_string());
    m.insert(4,  "\u{34}\u{fe0f}\u{20e3}".to_string());
    m.insert(5,  "\u{35}\u{fe0f}\u{20e3}".to_string());
    m.insert(6,  "\u{36}\u{fe0f}\u{20e3}".to_string());
    m.insert(7,  "\u{37}\u{fe0f}\u{20e3}".to_string());
    m.insert(8,  "\u{38}\u{fe0f}\u{20e3}".to_string());
    m.insert(9,  "\u{39}\u{fe0f}\u{20e3}".to_string());
    m.insert(10, "\u{01F51F}".to_string());
    m
  };
  // NUMBERS.insert
    // vec!["one", "two", "three", "four", "five", "six", "seven", "eight", "nine"];
}

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

  pub fn get_numbers(&self) -> HashMap<usize, String> {
    NUMBERS.to_owned()
  }

  pub fn to_number(&self, emoji: &ReactionType) -> Option<usize> {
    NUMBERS
      .iter()
      .find(|(_, v)| emoji.unicode_eq(v))
      .map(|(num, _)| *num)
  }
}
