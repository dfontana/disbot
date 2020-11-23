mod config;

use serenity::{
  async_trait,
  cache::Cache,
  client::bridge::gateway::GatewayIntents,
  model::{channel::Message, channel::ReactionType, gateway::Ready, guild::Emoji, id::GuildId},
  prelude::*,
};

use config::Config;

#[tokio::main]
async fn main() {
  let config = Config::new().expect("Err parsing environment");

  let mut client = Client::builder(&config.get_api_key())
    .add_intent(GatewayIntents::GUILDS)
    .add_intent(GatewayIntents::GUILD_EMOJIS)
    .add_intent(GatewayIntents::GUILD_MESSAGES)
    .add_intent(GatewayIntents::GUILD_MESSAGE_REACTIONS)
    .event_handler(Handler { config })
    .await
    .expect("Err creating client");

  if let Err(why) = client.start().await {
    println!("Client error: {:?}", why);
  }
}

struct Handler {
  config: Config,
}

impl Handler {
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
}

#[async_trait]
impl EventHandler for Handler {
  async fn message(&self, ctx: Context, msg: Message) {
    println!(
      "Got message {:?}",
      msg.content_safe(&ctx.cache).await.to_string()
    );
    if msg.is_own(&ctx.cache).await {
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
        .any(|cname| *cname == user.name.to_lowercase())
    });

    if (mentions_user.is_none()) {
      // Nothing to do here
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

  // reaction_add

  async fn ready(&self, _: Context, ready: Ready) {
    println!("{} is connected!", ready.user.name);
  }
}
