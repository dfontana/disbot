use super::SubCommandHandler;
use crate::cmd::arg_util::Args;
use anyhow::anyhow;
use rand::seq::SliceRandom;
use serenity::{
  all::CommandInteraction, async_trait, builder::EditInteractionResponse, client::Context,
};

#[derive(Default)]
pub struct Shuffle {}

#[async_trait]
impl SubCommandHandler for Shuffle {
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
      .expect("Songbird Voice client placed in at initialisation.");

    let handler_lock = manager
      .get(guild_id)
      .ok_or_else(|| anyhow!("I'm currently not in a voice channel"))?;
    let handler = handler_lock.lock().await;

    handler.queue().modify_queue(|f| {
      let front = f.pop_front();
      f.make_contiguous().shuffle(&mut rand::thread_rng());
      if let Some(v) = front {
        f.push_front(v);
      }
    });

    itx
      .edit_response(
        &ctx.http,
        EditInteractionResponse::new().content("Queue shuffled!"),
      )
      .await?;
    Ok(())
  }
}
