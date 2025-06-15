use super::SubCommandHandler;
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
    for (idx, _trk) in handler.queue().current_queue().iter().enumerate() {
      // TODO: Fix metadata retrieval for new songbird version
      // let typ = trk.typemap().lock().await;
      // let md = typ
      //   .get::<ListMetadata>()
      //   .expect("Guaranteed to exist from Play");
      body.push_str(&format!("{}. <UNKNOWN TITLE>\n", idx + 1));
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
