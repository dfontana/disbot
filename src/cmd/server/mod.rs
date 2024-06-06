mod ip;
mod list;
mod start;
mod stop;

use super::{AppInteractor, SubCommandHandler};
use crate::{docker::Docker, emoji::EmojiLookup};
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
  pub fn new(emoji: EmojiLookup, http: Client, docker: Docker) -> Self {
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
    let subopt = itx
      .data
      .options
      .first()
      .expect("Discord did not pass sub-opt");

    match subopt.name.as_str() {
      "start" => self.start.handle(ctx, itx, subopt).await?,
      "stop" => self.stop.handle(ctx, itx, subopt).await?,
      "list" => self.list.handle(ctx, itx, subopt).await?,
      "ip" => self.ip.handle(ctx, itx, subopt).await?,
      _ => unreachable!(),
    };

    Ok(())
  }
}
