use crate::cmd::SubCommandHandler;
use derive_new::new;
use serenity::{async_trait, client::Context};

#[derive(new)]
pub struct List {}

#[async_trait]
impl SubCommandHandler for List {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction,
    _subopt: &serenity::model::prelude::interaction::application_command::CommandDataOption,
  ) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
  }
}

// #[command]
// #[description = "List the servers that can be turned on"]
// #[usage = "list"]
// #[example = "list"]
// async fn list(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
//   exec_list(ctx, msg).await
// }

// #[instrument(name = "ServerList", level = "INFO", skip(ctx, msg))]
// async fn exec_list(ctx: &Context, msg: &Message) -> CommandResult {
//   if let Err(err) = Wol::inst()?.ensure_awake() {
//     info!("error {:?}", err);
//     msg.reply_ping(&ctx.http, err).await?;
//     return Ok(());
//   };

//   let docker = Docker::client()?.containers();

//   let containers = match docker.list().await {
//     Ok(c) => c,
//     Err(err) => {
//       error!("Failed to get containers - {:?}", err);
//       return Ok(());
//     }
//   };

//   for c in &containers {
//     info!("Data on hand - {:?}", c);
//     match docker.inspect(&c.id).await {
//       Ok(v) => {
//         info!("Inspected data - {:?}", v);
//       }
//       Err(err) => {
//         error!("Failed to inspect container - {:?}", err);
//         return Ok(());
//       }
//     }
//   }

//   // let id = &containers.get(0).unwrap().id;
//   // match docker.start(&id).await {
//   //   Err(err) => {
//   //     return Ok(());
//   //   },
//   //   _ => (),
//   // }
//   Ok(())
// }
