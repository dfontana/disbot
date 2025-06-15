use super::{play::ListMetadata, SubCommandHandler};
use crate::cmd::arg_util::Args;
use anyhow::anyhow;
use serenity::{
  all::CommandInteraction, async_trait, builder::EditInteractionResponse, client::Context,
  utils::MessageBuilder,
};

#[derive(Default)]
pub struct List {}

#[async_trait]
impl SubCommandHandler for List {
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

    let handler_lock = manager
      .get(guild_id)
      .ok_or_else(|| anyhow!("I'm currently not in a voice channel"))?;
    let handler = handler_lock.lock().await;

    let mut bld = MessageBuilder::new();
    bld.push_bold_line("Current Queue:");
    let mut body = String::new();
    for (idx, trk) in handler.queue().current_queue().iter().enumerate() {
      // Retrieve custom metadata using songbird 0.5.0 API
      // Note: data() will panic if type doesn't match, so we use a simple fallback approach
      let title = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        trk.data::<ListMetadata>().title.clone()
      })) {
        Ok(title) => title,
        Err(_) => "<UNKNOWN TITLE>".to_string(),
      };

      body.push_str(&format!("{}. {}\n", idx + 1, title));
    }

    itx
      .edit_response(
        &ctx.http,
        EditInteractionResponse::new().content(bld.push_codeblock(body, None).build()),
      )
      .await?;
    Ok(())
  }
}
