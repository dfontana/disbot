use std::error::Error;

use crate::emoji::EmojiLookup;

use super::SubCommandHandler;
use derive_new::new;
use serenity::{
  all::{CommandDataOption, CommandInteraction},
  async_trait,
  builder::EditInteractionResponse,
  client::Context,
  utils::MessageBuilder,
};

#[derive(new)]
pub struct Skip {
  emoji: EmojiLookup,
}

#[async_trait]
impl SubCommandHandler for Skip {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
    _subopt: &CommandDataOption,
  ) -> Result<(), Box<dyn Error>> {
    let guild_id = match itx.guild_id {
      Some(g) => g,
      None => {
        return Err("No Guild Id on Interaction".into());
      }
    };

    let manager = songbird::get(ctx)
      .await
      .expect("Songbird Voice client placed in at initialisation.")
      .clone();

    let emoji = self.emoji.get(&ctx.http, &ctx.cache, guild_id).await?;

    match manager.get(guild_id) {
      None => {
        itx
          .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("Not in a voice channel to play in"),
          )
          .await?;
      }
      Some(handler_lock) => {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.skip();
        itx
          .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(
              MessageBuilder::new()
                .push("I didn't like that song either ")
                .emoji(&emoji)
                .build(),
            ),
          )
          .await?;
      }
    }
    Ok(())
  }
}
