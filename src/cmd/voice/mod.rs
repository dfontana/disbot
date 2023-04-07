use std::error::Error;

use crate::{config::Config, emoji::EmojiLookup};

use self::connect_util::DisconnectHandle;

use super::{AppInteractor, SubCommandHandler};

mod connect_util;
mod list;
mod play;
mod reorder;
mod skip;
mod stop;

use list::*;
use play::*;
use reorder::*;
use serenity::{
  async_trait,
  builder::CreateApplicationCommands,
  client::Context,
  model::prelude::{
    command::{CommandOptionType, CommandType},
    interaction::{application_command::ApplicationCommandInteraction, InteractionResponseType},
  },
};
use skip::*;
use stop::*;
use tracing::{instrument, log::error};

const NAME: &str = "play";

pub struct Voice {
  play: Play,
  stop: Stop,
  skip: Skip,
  list: List,
  reorder: Reorder,
}

impl Voice {
  pub fn new(config: Config, emoji: EmojiLookup) -> Self {
    let disconnect = DisconnectHandle::new();
    Voice {
      play: Play::new(config, emoji.clone(), disconnect.clone()),
      stop: Stop::new(disconnect.clone()),
      skip: Skip::new(emoji.clone()),
      list: List::default(),
      reorder: Reorder::new(emoji),
    }
  }
}

#[async_trait]
impl AppInteractor for Voice {
  #[instrument(name = "Voice", level = "INFO", skip(self, commands))]
  fn register(&self, commands: &mut CreateApplicationCommands) {
    commands.create_application_command(|command| {
      command
        .name(NAME)
        .description("Sheebs Givith Loud Noises")
        .kind(CommandType::ChatInput)
        .create_option(|option| {
          option
            .kind(CommandOptionType::SubCommand)
            .name("yt")
            .description("Play sound from YT")
            .create_sub_option(|subopt| {
              subopt
                .kind(CommandOptionType::String)
                .name("link_or_search")
                .description("Link or search on YT")
                .required(true)
            })
        })
        .create_option(|option| {
          option
            .kind(CommandOptionType::SubCommand)
            .name("stop")
            .description("Kindly ask Shibba to stop screaming")
        })
        .create_option(|option| {
          option
            .kind(CommandOptionType::SubCommand)
            .name("skip")
            .description("Demand Shibba scream the next tune")
        })
        .create_option(|option| {
          option
            .kind(CommandOptionType::SubCommand)
            .name("list")
            .description("Shibba will reveal his inner secrets")
        })
        .create_option(|option| {
          option
            .kind(CommandOptionType::SubCommand)
            .name("reorder")
            .description("Move the given item to the given position in queue")
            .create_sub_option(|subopt| {
              subopt
                .kind(CommandOptionType::Integer)
                .name("from")
                .description("Item to move")
                .required(true)
                .min_int_value(1)
            })
            .create_sub_option(|subopt| {
              subopt
                .kind(CommandOptionType::Integer)
                .name("to")
                .description("Where to move to")
                .required(true)
                .min_int_value(1)
            })
        })
    });
  }

  #[instrument(name = "Voice", level = "INFO", skip(self, ctx, itx))]
  async fn app_interact(&self, ctx: &Context, itx: &ApplicationCommandInteraction) {
    let mut err = false;
    if let Err(e) = self._handle_app(ctx, itx).await {
      error!("Failed voice operation {:?}", e);
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

impl Voice {
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
      "yt" => self.play.handle(ctx, itx, subopt).await?,
      "stop" => self.stop.handle(ctx, itx, subopt).await?,
      "skip" => self.skip.handle(ctx, itx, subopt).await?,
      "reorder" => self.reorder.handle(ctx, itx, subopt).await?,
      "list" => self.list.handle(ctx, itx, subopt).await?,
      _ => unreachable!(),
    };

    Ok(())
  }
}
