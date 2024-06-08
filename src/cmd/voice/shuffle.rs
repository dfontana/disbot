use std::error::Error;

use super::SubCommandHandler;
use rand::seq::SliceRandom;
use serenity::{
  all::{CommandDataOption, CommandInteraction},
  async_trait,
  builder::EditInteractionResponse,
  client::Context,
};

#[derive(Default)]
pub struct Shuffle {}

#[async_trait]
impl SubCommandHandler for Shuffle {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
    _: &CommandDataOption,
  ) -> Result<(), Box<dyn Error>> {
    let guild_id = match itx.guild_id {
      Some(g) => g,
      None => {
        return Err("No Guild Id on Interaction".into());
      }
    };

    let manager = songbird::get(ctx)
      .await
      .expect("Songbird Voice client placed in at initialisation.");
    let handler_lock = match manager.get(guild_id) {
      None => {
        itx
          .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("I'm currently not in a voice channel"),
          )
          .await?;
        return Ok(());
      }
      Some(v) => v,
    };
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
