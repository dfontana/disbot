use crate::{
  cmd::{arg_util::Args, SubCommandHandler},
  docker::DockerClient,
};
use bollard::service::ContainerSummary;
use derive_new::new;
use itertools::Itertools;
use serenity::{
  all::CommandInteraction, async_trait, builder::EditInteractionResponse, client::Context,
  utils::MessageBuilder,
};
use std::sync::Arc;

#[derive(new)]
pub struct List {
  docker: Arc<Box<dyn DockerClient>>,
}

#[async_trait]
impl SubCommandHandler for List {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
    _args: &Args,
  ) -> Result<(), anyhow::Error> {
    let msg = match build_list_msg(&**self.docker).await {
      Ok(mut m) => m.build(),
      Err(e) => format!("Failed to list docker containers: {e}"),
    };
    itx
      .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
      .await?;
    Ok(())
  }
}

async fn build_list_msg(docker: &dyn DockerClient) -> Result<MessageBuilder, anyhow::Error> {
  let mut bdy = MessageBuilder::new();
  let summaries = docker.list().await?;
  let stat_len = 10;
  let max_len = summaries
    .iter()
    .map(extract_name)
    .map(|s| s.len())
    .max()
    .unwrap_or(1);

  let mut table = String::new();
  table.push_str(&format!("{:<max_len$} | {:<stat_len$}\n", "Name", "Status"));
  table.push_str(&format!("{:-<max_len$}---{:-<stat_len$}\n", "", ""));
  let msg = summaries
    .iter()
    .sorted_by_key(|summary| extract_name(summary))
    .fold(table, |mut acc, summary| {
      acc.push_str(&format!(
        "{:<max_len$} | {:<stat_len$}\n",
        extract_name(summary),
        summary
          .state
          .as_ref()
          .map(|s| format!("{s:?}"))
          .unwrap_or_else(|| "(No State)".into()),
      ));
      acc
    });
  bdy.push_codeblock(msg, None);
  Ok(bdy)
}

fn extract_name(summary: &ContainerSummary) -> &str {
  summary
    .names
    .as_ref()
    .and_then(|v| v.first())
    .and_then(|s| s.strip_prefix("/"))
    .unwrap_or("(No Name)")
}
