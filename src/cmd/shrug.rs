use crate::config::Config;
use serenity::{
  cache::Cache,
  model::{channel::Message, channel::ReactionType, guild::Emoji, id::GuildId},
  prelude::Context,
};

pub struct ShrugHandler {
  config: Config,
}

impl ShrugHandler {
  pub fn new(config: Config) -> Self {
    ShrugHandler { config }
  }
}

impl ShrugHandler {
  fn debug(&self, msg: &str) {
    if self.config.get_env().is_dev() {
      println!("{}", msg);
    }
  }

  async fn pull_emoji(&self, guild_id: GuildId, cache: &Cache) -> Result<Emoji, String> {
    // Pull the emoji from the guild attached to the message
    let maybe_emoji = cache
      .guild_field(guild_id, |guild| guild.emojis.clone())
      .await
      .ok_or("Failed to pull emojis for Guild".to_string())?;

    // If we do, though, we should find the emoji from the config
    let emoji = maybe_emoji
      .iter()
      .find_map(|(_, emoji)| {
        if emoji.name == *self.config.get_emote_name() {
          Some(emoji)
        } else {
          None
        }
      })
      .ok_or("Server does not have expected Emoji".to_string())?;

    Ok(emoji.clone())
  }

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

  pub async fn message(&self, ctx: &Context, msg: &Message) {
    if msg.is_own(&ctx.cache).await {
      self.debug("Skipping, self message");
      return;
    }

    let guild_id = match msg.guild_id {
      Some(id) => id,
      None => return,
    };

    let mentions_user = msg.mentions.iter().find(|user| {
      self
        .config
        .get_emote_users()
        .iter()
        .any(|cname| *cname.to_lowercase() == user.name.to_lowercase())
    });

    if mentions_user.is_none() {
      self.debug("Did not find a matching user mention");
      return;
    }

    let emoji = self.pull_emoji(guild_id, &ctx.cache).await;

    let send = match emoji {
      Ok(e) => self.react_and_send(e, &ctx, &msg).await,
      Err(cause) => Err(cause),
    };

    match send {
      Ok(_) => return,
      Err(cause) => {
        println!("Failed to react {:?}", cause);
        if let Err(why) = msg
          .channel_id
          .say(&ctx.http, "You taketh my shrug, you taketh me :(")
          .await
        {
          println!("Failed to send error {:?}", why);
          return;
        }
      }
    }
  }
}
