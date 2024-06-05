mod ip;
mod list;
mod start;
mod stop;

use std::error::Error;

use ip::*;
use list::*;
use reqwest::Client;
use serenity::{
  async_trait,
  builder::CreateApplicationCommands,
  model::prelude::{
    command::{CommandOptionType, CommandType},
    interaction::{application_command::ApplicationCommandInteraction, InteractionResponseType},
  },
  prelude::Context,
};
use start::*;
use stop::*;

use tracing::{error, instrument};

use crate::{docker::Docker, emoji::EmojiLookup};

use super::{AppInteractor, SubCommandHandler};

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
      list: List::new(docker),
      start: Start::new(),
      stop: Stop::new(),
      ip: Ip::new(http, emoji),
    }
  }
}

#[async_trait]
impl AppInteractor for GameServers {
  #[instrument(name = "Servers", level = "INFO", skip(self, commands))]
  fn register(&self, commands: &mut CreateApplicationCommands) {
    commands.create_application_command(|command| {
      command
        .name(NAME)
        .description("Sheebs Thinks He's IT")
        .kind(CommandType::ChatInput)
        .create_option(|option| {
          option
            .kind(CommandOptionType::SubCommand)
            .name("ip")
            .description("Shibba knows where he is, do you?")
        })
        .create_option(|option| {
          option
            .kind(CommandOptionType::SubCommand)
            .name("stop")
            .description("Make Shibba stop a server")
            .create_sub_option(|subopt| {
              subopt
                .kind(CommandOptionType::String)
                .name("server-name")
                .description("name of server from list command")
                .required(true)
            })
        })
        .create_option(|option| {
          option
            .kind(CommandOptionType::SubCommand)
            .name("start")
            .description("Make Shibba start a server")
            .create_sub_option(|subopt| {
              subopt
                .kind(CommandOptionType::String)
                .name("server-name")
                .description("Name of server from list command")
                .required(true)
            })
        })
        .create_option(|option| {
          option
            .kind(CommandOptionType::SubCommand)
            .name("list")
            .description("Shibba will take a guess at what servers exist")
        })
    });
  }

  #[instrument(name = "Servers", level = "INFO", skip(self, ctx, itx))]
  async fn app_interact(&self, ctx: &Context, itx: &ApplicationCommandInteraction) {
    let mut err = false;
    if let Err(e) = self._handle_app(ctx, itx).await {
      error!("Failed server operation {:?}", e);
      err = true;
    }
    if err {
      if let Err(e) = itx
        .edit_original_interaction_response(&ctx.http, |f| f.content("Command failed"))
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
    itx: &ApplicationCommandInteraction,
  ) -> Result<(), Box<dyn Error>> {
    if !itx.data.name.as_str().eq(NAME) {
      return Ok(());
    }
    itx
      .create_interaction_response(&ctx.http, |bld| {
        bld
          .kind(InteractionResponseType::ChannelMessageWithSource)
          .interaction_response_data(|f| f.content("Loading..."))
      })
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
      // TODO
      // "start" => self.start.handle(ctx, itx, subopt).await?,
      // "stop" => self.stop.handle(ctx, itx, subopt).await?,
      "list" => self.list.handle(ctx, itx, subopt).await?,
      "ip" => self.ip.handle(ctx, itx, subopt).await?,
      _ => unreachable!(),
    };

    Ok(())
  }
}
