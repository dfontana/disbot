// use serenity::framework::standard::macros::group;

use std::error::Error;

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
  model::interactions::{
    application_command::{
      ApplicationCommandInteraction, ApplicationCommandOptionType, ApplicationCommandType,
    },
    InteractionResponseType,
  },
};
use skip::*;
use stop::*;
use tracing::{instrument, log::error};

const NAME: &'static str = "play";

#[derive(Default)]
pub struct Voice {
  play: Play,
  stop: Stop,
  skip: Skip,
  list: List,
  reorder: Reorder,
}

#[async_trait]
impl AppInteractor for Voice {
  #[instrument(name = "Voice", level = "INFO", skip(self, commands))]
  fn register(&self, commands: &mut CreateApplicationCommands) {
    commands.create_application_command(|command| {
      command
        .name(NAME)
        .description("Sheebs Givith Loud Noises")
        .kind(ApplicationCommandType::ChatInput)
        .create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::SubCommand)
            .name("yt")
            .description("Play sound from YT")
            .create_sub_option(|subopt| {
              subopt
                .kind(ApplicationCommandOptionType::String)
                .name("link_or_search")
                .description("Link or search on YT")
                .required(true)
            })
        })
        .create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::SubCommand)
            .name("stop")
            .description("Kindly ask Shibba to stop screaming")
        })
        .create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::SubCommand)
            .name("skip")
            .description("Demand Shibba scream the next tune")
        })
        .create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::SubCommand)
            .name("list")
            .description("Shibba will reveal his inner secrets")
        })
        .create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::SubCommand)
            .name("reorder")
            .description("Move the given item to the given position in queue")
            .create_sub_option(|subopt| {
              subopt
                .kind(ApplicationCommandOptionType::Integer)
                .name("from")
                .description("Item to move")
                .required(true)
                .min_int_value(1)
            })
            .create_sub_option(|subopt| {
              subopt
                .kind(ApplicationCommandOptionType::Integer)
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
      .iter()
      .next()
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
