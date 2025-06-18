mod ip;
mod list;
mod start;
mod stop;

use super::{arg_util::Args, AppInteractor, SubCommandHandler};
use crate::{docker::DockerClient, emoji::EmojiLookup};
use ip::*;
use list::*;
use reqwest::Client;
use serenity::{
  all::{CommandInteraction, CommandOptionType, CommandType, CreateCommandOption},
  async_trait,
  builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage},
  prelude::Context,
};
use start::*;
use std::error::Error;
use stop::*;
use tracing::{error, instrument};

const NAME: &str = "servers";

pub struct GameServers {
  list: List,
  start: Start,
  stop: Stop,
  ip: Ip,
}

impl GameServers {
  pub fn new(emoji: EmojiLookup, http: Client, docker: Box<dyn DockerClient>) -> Self {
    use std::sync::Arc;
    let docker = Arc::new(docker);
    GameServers {
      list: List::new(docker.clone()),
      start: Start::new(docker.clone()),
      stop: Stop::new(docker),
      ip: Ip::new(http, emoji),
    }
  }
}

#[async_trait]
impl AppInteractor for GameServers {
  #[instrument(name = "Servers", level = "INFO", skip(self))]
  fn commands(&self) -> Vec<CreateCommand> {
    vec![CreateCommand::new(NAME)
      .description("Sheebs Thinks He's IT")
      .kind(CommandType::ChatInput)
      .add_option(CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "ip",
        "Shibba knows where he is, do you?",
      ))
      .add_option(CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "list",
        "Shibba will take a guess at what servers exist",
      ))
      .add_option(
        CreateCommandOption::new(
          CommandOptionType::SubCommand,
          "stop",
          "Make Shibba stop a server",
        )
        .add_sub_option(
          CreateCommandOption::new(
            CommandOptionType::String,
            "server-name",
            "name of server from list command",
          )
          .required(true),
        ),
      )
      .add_option(
        CreateCommandOption::new(
          CommandOptionType::SubCommand,
          "start",
          "Make Shibba start a server",
        )
        .add_sub_option(
          CreateCommandOption::new(
            CommandOptionType::String,
            "server-name",
            "name of server from list command",
          )
          .required(true),
        ),
      )]
  }

  #[instrument(name = "Servers", level = "INFO", skip(self, ctx, itx))]
  async fn app_interact(&self, ctx: &Context, itx: &CommandInteraction) {
    let mut err = false;
    if let Err(e) = self._handle_app(ctx, itx).await {
      error!("Failed server operation {:?}", e);
      err = true;
    }
    if err {
      if let Err(e) = itx
        .create_response(
          &ctx.http,
          CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("Command failed"),
          ),
        )
        .await
      {
        error!("Failed to notify app failed {:?}", e);
      }
    }
  }
}

impl GameServers {
  async fn _handle_app(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
  ) -> Result<(), Box<dyn Error>> {
    if !itx.data.name.as_str().eq(NAME) {
      return Ok(());
    }
    itx
      .create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
          CreateInteractionResponseMessage::new().content("Loading..."),
        ),
      )
      .await?;

    // This is a bit annoying of an interface but when we're talking
    // subcommands here the options vec should only ever be 1 long
    // and its gonna have the option on it.
    let top_args = itx.data.options();
    let subopt = top_args.first().expect("Discord did not pass sub-opt");
    let args = match &subopt.value {
      serenity::all::ResolvedValue::SubCommand(c) => Args::from(c),
      _ => return Err("Dev error - subopt registered that's not a subcommand".into()),
    };

    match subopt.name {
      "start" => self.start.handle(ctx, itx, &args).await?,
      "stop" => self.stop.handle(ctx, itx, &args).await?,
      "list" => self.list.handle(ctx, itx, &args).await?,
      "ip" => self.ip.handle(ctx, itx, &args).await?,
      _ => unreachable!(),
    };

    Ok(())
  }
}
