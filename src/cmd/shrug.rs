use crate::config::Config;
use crate::debug::Debug;
use crate::emoji::EmojiLookup;
use serenity::{
  model::{channel::Message, channel::ReactionType, guild::Emoji},
  prelude::Context,
};

pub struct ShrugHandler {
  config: Config,
  debug: Debug,
}

impl ShrugHandler {
  pub fn new(config: Config, debug: Debug) -> Self {
    ShrugHandler { config, debug }
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
      self.debug.log("Skipping, self message");
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
      self.debug.log("Did not find a matching user mention");
      return;
    }

    let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await;

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
