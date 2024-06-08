use super::SubCommandHandler;
use crate::{cmd::arg_util::Args, emoji::EmojiLookup};
use anyhow::anyhow;
use derive_new::new;
use serenity::{
  all::CommandInteraction, async_trait, builder::EditInteractionResponse, client::Context,
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
    _: &Args,
  ) -> Result<(), anyhow::Error> {
    let guild_id = itx
      .guild_id
      .ok_or_else(|| anyhow!("No Guild Id on Interaction"))?;

    let manager = songbird::get(ctx)
      .await
      .expect("Songbird Voice client placed in at initialisation.")
      .clone();

    let emoji = self.emoji.get(&ctx.http, &ctx.cache, guild_id).await?;

    let handler_lock = manager
      .get(guild_id)
      .ok_or_else(|| anyhow!("Not in a voice channel to play in"))?;
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

    Ok(())
  }
}
