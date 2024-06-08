use std::{collections::HashMap, error::Error};

use super::SubCommandHandler;
use crate::emoji::EmojiLookup;
use derive_new::new;
use serenity::{
  all::{CommandDataOption, CommandInteraction, ResolvedValue},
  async_trait,
  builder::EditInteractionResponse,
  client::Context,
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
    _subopt: &CommandDataOption,
  ) -> Result<(), Box<dyn Error>> {
    // 2 args: from, to. Min value 1. Integers.
    let args: HashMap<String, _> = itx
      .data
      .options()
      .iter()
      .map(|d| (d.name.to_owned(), d.value.to_owned()))
      .collect();

    // Get the handler
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
    let handler_lock = match manager.get(guild_id) {
      None => {
        itx
          .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("Not in a voice channel"),
          )
          .await?;
        return Ok(());
      }
      Some(v) => v,
    };
    let handler = handler_lock.lock().await;

    //  Validate the position args
    let queue_size = handler.queue().current_queue().len();
    let posa = match validate_position(get_arg(&args, "from"), queue_size) {
      Ok(v) => v,
      Err(e) => {
        itx
          .edit_response(&ctx.http, EditInteractionResponse::new().content(&e))
          .await?;
        return Ok(());
      }
    };
    let posb = match validate_position(get_arg(&args, "to"), queue_size) {
      Ok(v) => v,
      Err(e) => {
        itx
          .edit_response(&ctx.http, EditInteractionResponse::new().content(&e))
          .await?;
        return Ok(());
      }
    };
    if posa == posb {
      itx
        .edit_response(
          &ctx.http,
          EditInteractionResponse::new().content("A touch psychotic are we?"),
        )
        .await?;
      return Ok(());
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

    let emoji = self.emoji.get(&ctx.http, &ctx.cache, guild_id).await?;
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

fn get_arg(args: &HashMap<String, ResolvedValue>, key: &str) -> Result<usize, String> {
  args
    .get(key)
    .and_then(|d| match d {
      ResolvedValue::Integer(v) => Some(v.to_owned()),
      _ => None,
    })
    .map(|i| i as usize)
    .ok_or_else(|| "Missing bound".into())
}

fn validate_position<T>(maybe_pos: Result<usize, T>, queue_size: usize) -> Result<usize, String> {
  let pos = match maybe_pos {
    Err(_) => return Err("Must provide a numeric position".into()),
    Ok(v) => v,
  };
  if pos <= 1 {
    return Err("Cannot move first item".into());
  }
  if pos > queue_size {
    return Err("Can only move item to end of queue".into());
  }
  Ok(pos)
}
