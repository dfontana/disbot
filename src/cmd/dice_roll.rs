use super::{arg_util::Args, AppInteractor};
use crate::emoji::EmojiLookup;
use anyhow::anyhow;
use derive_new::new;
use rand::Rng;
use serenity::{
  all::{CommandInteraction, CommandOptionType, CommandType, CreateCommandOption},
  async_trait,
  builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse,
  },
  client::Context,
  utils::MessageBuilder,
};
use std::error::Error;
use tracing::{instrument, log::error};

const NAME: &str = "roll";

#[derive(new)]
pub struct DiceRoll {
  emoji: EmojiLookup,
}

#[async_trait]
impl AppInteractor for DiceRoll {
  #[instrument(name = NAME, level = "INFO", skip(self))]
  fn commands(&self) -> Vec<CreateCommand> {
    vec![CreateCommand::new(NAME)
      .description("Roll a die, optionally between the given bounds")
      .kind(CommandType::ChatInput)
      .add_option(
        CreateCommandOption::new(CommandOptionType::Integer, "lower", "Above or equal to")
          .min_int_value(0)
          .max_int_value(100)
          .required(false),
      )
      .add_option(
        CreateCommandOption::new(CommandOptionType::Integer, "upper", "Below or equal to")
          .min_int_value(0)
          .max_int_value(100)
          .required(false),
      )]
  }

  #[instrument(name = NAME, level = "INFO", skip(self, ctx, itx))]
  async fn app_interact(&self, ctx: &Context, itx: &CommandInteraction) {
    let mut err = false;
    if let Err(e) = self._handle_app(ctx, itx).await {
      error!("Failed to roll {:?}", e);
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

impl DiceRoll {
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
          CreateInteractionResponseMessage::new().content("Rolling..."),
        ),
      )
      .await?;
    let raw_opts = itx.data.options();
    let args = Args::from(&raw_opts);
    let (lower, upper) = match validate(&args) {
      Ok(v) => v,
      Err(e) => {
        itx
          .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!("{}", e)),
          )
          .await?;
        return Ok(());
      }
    };
    let roll = rand::rng().random_range(lower..upper + 1);
    let guild_id = match itx.guild_id {
      Some(id) => id,
      None => return Ok(()),
    };
    let emoji = self.emoji.get(&ctx.http, guild_id).await?;
    let mut response = MessageBuilder::new();
    response
      .push(format!("<@{}>", itx.user.id))
      .push(" rolls ")
      .push(" ")
      .emoji(&emoji)
      .push(" ");

    match roll {
      1 => response.emoji(&emoji),
      21 => response.push_bold("21 - you stupid!"),
      47 => response.push_bold("god damn 47"),
      69 => response.push_bold("69").push_italic("...nice"),
      _ => response.push_bold(roll.to_string()),
    };

    let resp_string = response
      .push(" ")
      .emoji(&emoji)
      .push(" ")
      .push_mono(format!("({} - {})", lower, upper))
      .build();

    itx
      .edit_response(
        &ctx.http,
        EditInteractionResponse::new().content(resp_string),
      )
      .await?;

    Ok(())
  }
}

fn validate(args: &Args) -> Result<(u32, u32), anyhow::Error> {
  match args.len() {
    0 => Ok((1, 100)),
    1 => Ok((
      args.i64("lower").or(args.i64("upper")).map(|v| *v as u32)?,
      100,
    )),
    2 => {
      let lower = args.i64("lower").map(|v| *v as u32)?;
      let upper = args.i64("upper").map(|v| *v as u32)?;
      if upper < lower {
        Ok((upper, lower))
      } else {
        Ok((lower, upper))
      }
    }
    _ => Err(anyhow!("Too many arugments provided")),
  }
}
