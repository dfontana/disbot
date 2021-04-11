use crate::{cmd::server::wol::Wol, debug::Debug, docker::Docker};
use serenity::{
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
};

#[command]
#[description = "List the servers that can be turned on"]
#[usage = "list"]
#[example = "list"]
async fn list(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
  match Wol::inst()?.ensure_awake() {
    Err(err) => {
      msg.reply_ping(&ctx.http, err).await?;
      return Ok(());
    }
    _ => (),
  };

  let docker = Docker::client()?.containers();

  let containers = match docker.list().await {
    Ok(c) => c,
    Err(err) => {
      Debug::inst("server_list").log(&format!("Failed to get containers - {:?}", err));
      return Ok(());
    }
  };

  for c in &containers {
    println!("Data on hand - {:?}", c);
    match docker.inspect(&c.id).await {
      Ok(v) => {
        println!("Inspected data - {:?}", v);
      }
      Err(err) => {
        Debug::inst("server_list").log(&format!("Failed to inspect container - {:?}", err));
        return Ok(());
      }
    }
  }

  // println!("Starting first container!");
  // let id = &containers.get(0).unwrap().id;
  // match docker.start(&id).await {
  //   Err(err) => {
  //     Debug::inst("server_list").log(&format!("Failed to inspect container - {:?}", err));
  //     return Ok(());
  //   },
  //   _ => (),
  // }
  // println!("Done!");

  Ok(())
}
