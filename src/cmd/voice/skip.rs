use std::error::Error;

use crate::emoji::EmojiLookup;

use super::SubCommandHandler;
use serenity::{
  async_trait,
  client::Context,
  model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandInteractionDataOption,
  },
  utils::MessageBuilder,
};

#[derive(Default)]
pub struct Skip {}

#[async_trait]
impl SubCommandHandler for Skip {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &ApplicationCommandInteraction,
    _subopt: &ApplicationCommandInteractionDataOption,
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

    let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;

    match manager.get(guild_id) {
      None => {
        itx
          .create_followup_message(&ctx.http, |f| {
            f.content("Not in a voice channel to play in")
          })
          .await?;
      }
      Some(handler_lock) => {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.skip();
        itx
          .create_followup_message(&ctx.http, |f| {
            f.content(
              MessageBuilder::new()
                .push("I didn't like that song either ")
                .mention(&emoji)
                .build(),
            )
          })
          .await?;
      }
    }
    Ok(())
  }
}
