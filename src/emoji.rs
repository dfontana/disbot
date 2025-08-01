use crate::config::Config;
use anyhow::anyhow;
use base64::{engine::general_purpose, Engine as _};
use cached::proc_macro::cached;
use once_cell::sync::Lazy;
use serenity::{
  http::Http,
  model::{guild::Emoji, id::GuildId},
};
use tracing::info;

static EMOJI_IMAGE: Lazy<String> = Lazy::new(|| {
  format!(
    "data:image/png;base64,{}",
    general_purpose::STANDARD.encode(include_bytes!("img/shrug-cat.png"))
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

  pub async fn get(&self, http: &Http, guild_id: GuildId) -> Result<Emoji, anyhow::Error> {
    get_emoji(http, guild_id, self.emote_name.clone()).await
  }
}

#[cached(
  time = 600,
  result = true,
  key = "String",
  convert = r##"{format!("{}:{}", guild_id, name)}"##
)]
async fn get_emoji(http: &Http, guild_id: GuildId, name: String) -> Result<Emoji, anyhow::Error> {
  // Check if the guild has the emoji registered (have to search by name, not id)
  // return if they do (we don't want to re-create it)
  let emojis = guild_id.emojis(http).await?;
  if let Some(emote) = emojis.iter().find(|em| em.name == name) {
    info!("Resolved emoji {} for guild {}", name, guild_id);
    return Ok(emote.clone());
  }

  // Otherwise they don't have it, so let's make it for them
  info!("Registering emoji {} for guild {}", name, guild_id);
  let emote = guild_id
    .create_emoji(http, &name, &EMOJI_IMAGE)
    .await
    .map_err(|err| anyhow!(err))?;
  Ok(emote)
}
