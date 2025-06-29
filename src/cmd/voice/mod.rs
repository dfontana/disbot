mod connect_util;
mod list;
mod play;
mod reorder;
mod shuffle;
mod skip;
mod stop;

use self::connect_util::{DisconnectActor, DisconnectMessage};
use super::arg_util::Args;
use super::{AppInteractor, SubCommandHandler};
use crate::shutdown::ShutdownCoordinator;
use crate::{actor::ActorHandle, config::Config, emoji::EmojiLookup};
use list::*;
use play::*;
use reorder::*;
use serenity::all::{CommandOptionType, CommandType, CreateCommand, CreateCommandOption};
use serenity::builder::{
  CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse,
};
use serenity::{all::CommandInteraction, async_trait, client::Context};
use shuffle::*;
use skip::*;
use stop::*;
use tracing::{error, instrument};

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
  pub fn new(config: Config, emoji: EmojiLookup, shutdown: &mut ShutdownCoordinator) -> Self {
    let disconnect =
      ActorHandle::<DisconnectMessage>::spawn(|r, _| Box::new(DisconnectActor::new(r)), shutdown);
    Self {
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
      .description("Binkies Givith Loud Noises")
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
        "Kindly ask Binkies to stop screaming",
      ))
      .add_option(CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "skip",
        "Demand Binkies scream the next tune",
      ))
      .add_option(CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "list",
        "Binkies will reveal his inner secrets",
      ))
      .add_option(CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "shuffle",
        "Binkies will throw the queue on the ground",
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
    if !itx.data.name.as_str().eq(NAME) {
      return;
    }

    if let Err(e) = itx
      .create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
          CreateInteractionResponseMessage::new().content("Loading..."),
        ),
      )
      .await
    {
      error!("{:?}", e);
      return;
    }

    // This is a bit annoying of an interface but when we're talking
    // subcommands here the options vec should only ever be 1 long
    // and its gonna have the option on it.
    let top_args = itx.data.options();
    let subopt = top_args.first().expect("Discord did not pass sub-opt");
    let args = match &subopt.value {
      serenity::all::ResolvedValue::SubCommand(c) => Args::from(c),
      _ => unreachable!("Dev error - subopt was not subcommand"),
    };

    if let Err(e) = match subopt.name {
      "yt" => self.play.handle(ctx, itx, &args).await,
      "stop" => self.stop.handle(ctx, itx, &args).await,
      "skip" => self.skip.handle(ctx, itx, &args).await,
      "reorder" => self.reorder.handle(ctx, itx, &args).await,
      "list" => self.list.handle(ctx, itx, &args).await,
      "shuffle" => self.shuffle.handle(ctx, itx, &args).await,
      _ => unreachable!(),
    } {
      error!("{:?}", e);
      let _ = itx
        .edit_response(
          &ctx.http,
          EditInteractionResponse::new().content(format!("{}", e)),
        )
        .await;
    }
  }
}
