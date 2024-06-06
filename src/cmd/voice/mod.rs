mod connect_util;
mod list;
mod play;
mod reorder;
mod shuffle;
mod skip;
mod stop;

use self::connect_util::{DisconnectActor, DisconnectMessage};
use super::{AppInteractor, SubCommandHandler};
use crate::{actor::ActorHandle, config::Config, emoji::EmojiLookup};
use list::*;
use play::*;
use reorder::*;
use serenity::all::{CommandOptionType, CommandType, CreateCommand, CreateCommandOption};
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::{all::CommandInteraction, async_trait, client::Context};
use shuffle::*;
use skip::*;
use std::error::Error;
use stop::*;
use tracing::{instrument, log::error};

const NAME: &str = "play";

pub struct Voice {
  play: Play,
  stop: Stop,
  skip: Skip,
  shuffle: Shuffle,
  list: List,
  reorder: Reorder,
}

impl Voice {
  pub fn new(config: Config, emoji: EmojiLookup) -> Self {
    let disconnect =
      ActorHandle::<DisconnectMessage>::spawn(|r, _| Box::new(DisconnectActor::new(r)));
    Voice {
      play: Play::new(config, emoji.clone(), disconnect.clone()),
      stop: Stop::new(disconnect),
      skip: Skip::new(emoji.clone()),
      shuffle: Shuffle::default(),
      list: List::default(),
      reorder: Reorder::new(emoji),
    }
  }
}

#[async_trait]
impl AppInteractor for Voice {
  #[instrument(name = "Voice", level = "INFO", skip(self))]
  fn commands(&self) -> Vec<CreateCommand> {
    vec![CreateCommand::new(NAME)
      .description("Sheebs Givith Loud Noises")
      .kind(CommandType::ChatInput)
      .add_option(
        CreateCommandOption::new(CommandOptionType::SubCommand, "yt", "Play sound from YT")
          .add_sub_option(
            CreateCommandOption::new(
              CommandOptionType::String,
              "link_or_search",
              "Link or search on YT",
            )
            .required(true),
          ),
      )
      .add_option(CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "stop",
        "Kindly ask Shibba to stop screaming",
      ))
      .add_option(CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "skip",
        "Demand Shibba scream the next tune",
      ))
      .add_option(CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "list",
        "Shibba will reveal his inner secrets",
      ))
      .add_option(CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "shuffle",
        "Shibba will throw the queue on the ground",
      ))
      .add_option(
        CreateCommandOption::new(
          CommandOptionType::SubCommand,
          "reorder",
          "Move the given item to the given position in queue",
        )
        .add_sub_option(
          CreateCommandOption::new(CommandOptionType::Integer, "from", "Item to move")
            .required(true)
            .min_int_value(1),
        )
        .add_sub_option(
          CreateCommandOption::new(CommandOptionType::Integer, "to", "Where to move to")
            .required(true)
            .min_int_value(1),
        ),
      )]
  }

  #[instrument(name = "Voice", level = "INFO", skip(self, ctx, itx))]
  async fn app_interact(&self, ctx: &Context, itx: &CommandInteraction) {
    let mut err = false;
    if let Err(e) = self._handle_app(ctx, itx).await {
      error!("Failed voice operation {:?}", e);
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

impl Voice {
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
      "yt" => self.play.handle(ctx, itx, subopt).await?,
      "stop" => self.stop.handle(ctx, itx, subopt).await?,
      "skip" => self.skip.handle(ctx, itx, subopt).await?,
      "reorder" => self.reorder.handle(ctx, itx, subopt).await?,
      "list" => self.list.handle(ctx, itx, subopt).await?,
      "shuffle" => self.shuffle.handle(ctx, itx, subopt).await?,
      _ => unreachable!(),
    };

    Ok(())
  }
}
