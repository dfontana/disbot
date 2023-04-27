use std::error::Error;

use super::SubCommandHandler;
use rand::seq::SliceRandom;
use serenity::{
  async_trait,
  client::Context,
  model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption,
  },
};

#[derive(Default)]
pub struct Shuffle {}

#[async_trait]
impl SubCommandHandler for Shuffle {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &ApplicationCommandInteraction,
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
          .edit_original_interaction_response(&ctx.http, |f| {
            f.content("I'm currently not in a voice channel")
          })
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
      .edit_original_interaction_response(&ctx.http, |f| f.content("Queue shuffled!"))
      .await?;
    Ok(())
  }
}
