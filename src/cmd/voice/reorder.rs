use super::SubCommandHandler;
use crate::{cmd::arg_util::Args, emoji::EmojiLookup};
use anyhow::anyhow;
use derive_new::new;
use serenity::{
  all::CommandInteraction, async_trait, builder::EditInteractionResponse, client::Context,
  utils::MessageBuilder,
};

#[derive(new)]
pub struct Reorder {
  emoji: EmojiLookup,
}

#[async_trait]
impl SubCommandHandler for Reorder {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
    args: &Args,
  ) -> Result<(), anyhow::Error> {
    let guild_id = itx
      .guild_id
      .ok_or_else(|| anyhow!("No Guild Id on Interaction"))?;
    let manager = songbird::get(ctx)
      .await
      .expect("Songbird Voice client placed in at initialisation.")
      .clone();
    let handler_lock = manager
      .get(guild_id)
      .ok_or_else(|| anyhow!("Not in a voice channel"))?;
    let handler = handler_lock.lock().await;

    // 2 args: from, to. Min value 1. Integers.
    // Validate the position args
    let queue_size = handler.queue().current_queue().len();
    let posa = validate_position(args.i64("from"), queue_size)?;
    let posb = validate_position(args.i64("to"), queue_size)?;
    if posa == posb {
      return Err(anyhow!("A touch psychotic are we?"));
    }

    // Perform the movement
    handler.queue().modify_queue(|queue| {
      if let Some(item) = queue.remove(posa - 1) {
        // b/c queue is 0 based
        if posb - 1 < queue.len() {
          queue.insert(posb - 1, item);
        } else {
          queue.push_back(item);
        }
      }
    });

    let emoji = self.emoji.get(&ctx.http, guild_id).await?;
    itx
      .edit_response(
        &ctx.http,
        EditInteractionResponse::new().content(
          MessageBuilder::new()
            .emoji(&emoji)
            .push_bold("Queued updated!")
            .emoji(&emoji)
            .push_line("")
            .push_italic("You can list the queue your damn self")
            .build(),
        ),
      )
      .await?;

    Ok(())
  }
}

fn validate_position(
  maybe_pos: Result<&i64, anyhow::Error>,
  queue_size: usize,
) -> Result<usize, anyhow::Error> {
  let pos = match maybe_pos {
    Err(e) => return Err(anyhow!("Must provide a numeric position").context(e)),
    Ok(v) => v,
  };
  if *pos <= 1 {
    return Err(anyhow!("Cannot move first item"));
  }
  let posb = *pos as usize;
  if posb > queue_size {
    return Err(anyhow!("Can only move item to end of queue"));
  }
  Ok(posb)
}
