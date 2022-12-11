use std::error::Error;

use super::SubCommandHandler;
use serenity::{
  async_trait,
  client::Context,
  model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption,
  },
  utils::MessageBuilder,
};

#[derive(Default)]
pub struct List {}

#[async_trait]
impl SubCommandHandler for List {
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
      .expect("Songbird Voice client placed in at initialisation.")
      .clone();
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

    let mut bld = MessageBuilder::new();
    bld.push_bold_line("Current Queue:");
    let mut body = String::new();
    for (idx, trk) in handler.queue().current_queue().iter().enumerate() {
      body.push_str(&format!(
        "{}. '{}'\n",
        idx + 1,
        trk
          .metadata()
          .track
          .as_ref()
          .or_else(|| trk.metadata().title.as_ref())
          .unwrap_or(&"<UNKNOWN>".to_string())
      ));
    }

    itx
      .edit_original_interaction_response(&ctx.http, |f| {
        f.content(bld.push_codeblock(body, None).build())
      })
      .await?;
    Ok(())
  }
}
