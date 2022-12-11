use crate::config::Config;

use cached::proc_macro::cached;
use once_cell::sync::Lazy;
use serenity::{
  cache::Cache,
  http::Http,
  model::{guild::Emoji, id::GuildId},
};
use tracing::info;

static EMOJI_IMAGE: Lazy<String> = Lazy::new(|| {
  format!(
    "data:image/png;base64,{}",
    base64::encode(include_bytes!("img/shrug-dog.png"))
  )
});

#[derive(Clone)]
pub struct EmojiLookup {
  emote_name: String,
}

impl EmojiLookup {
  pub fn new(config: &Config) -> Self {
    EmojiLookup {
      emote_name: config.emote_name.to_string(),
    }
  }

  pub async fn get(&self, http: &Http, cache: &Cache, guild_id: GuildId) -> Result<Emoji, String> {
    get_emoji(http, cache, guild_id, self.emote_name.clone()).await
  }
}

#[cached(
  time = 600,
  result = true,
  sync_writes = true,
  key = "String",
  convert = r##"{format!("{}:{}", guild_id.0, name)}"##
)]
async fn get_emoji(
  http: &Http,
  scache: &Cache,
  guild_id: GuildId,
  name: String,
) -> Result<Emoji, String> {
  // Check if the guild has the emoji registered (have to search by name, not id)
  // return if they do (we don't want to re-create it)
  let emojis = scache
    .guild_field(guild_id, |guild| guild.emojis.clone())
    .ok_or_else(|| "Failed to pull emojis for Guild".to_string())?;
  if let Some((_, emote)) = emojis.iter().find(|(_, em)| em.name == name) {
    info!("Resolved emoji {} for guild {}", name, guild_id.0);
    return Ok(emote.clone());
  }

  // Otherwise they don't have it, so let's make it for them
  info!("Registering emoji {} for guild {}", name, guild_id.0);
  let emote = guild_id
    .create_emoji(http, &name, &EMOJI_IMAGE)
    .await
    .map_err(|err| format!("{:?}", err))?;
  Ok(emote)
}
