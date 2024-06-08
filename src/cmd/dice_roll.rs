use super::AppInteractor;
use crate::emoji::EmojiLookup;
use derive_new::new;
use rand::Rng;
use serenity::{
  all::{
    CommandInteraction, CommandOptionType, CommandType, CreateCommandOption, ResolvedOption,
    ResolvedValue,
  },
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
  #[instrument(name = "Roll", level = "INFO", skip(self))]
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

  #[instrument(name = "Roll", level = "INFO", skip(self, ctx, itx))]
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

    let (lower, upper) = match validate(&itx.data.options()) {
      Ok(v) => v,
      Err(e) => {
        itx
          .edit_response(&ctx.http, EditInteractionResponse::new().content(e))
          .await?;
        return Ok(());
      }
    };
    let roll = rand::thread_rng().gen_range(lower..upper + 1);
    let guild_id = match itx.guild_id {
      Some(id) => id,
      None => return Ok(()),
    };
    let emoji = self.emoji.get(&ctx.http, &ctx.cache, guild_id).await?;
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

fn validate(args: &Vec<ResolvedOption>) -> Result<(u32, u32), String> {
  match args.len() {
    0 => Ok((1, 100)),
    1 => Ok((extract_int(args, 0)?, 100)),
    2 => {
      let lower = extract_int(args, 0)?;
      let upper = extract_int(args, 1)?;
      if upper < lower {
        Ok((upper, lower))
      } else {
        Ok((lower, upper))
      }
    }
    _ => Err("Too many arugments provided".to_string()),
  }
}

fn extract_int(args: &[ResolvedOption], idx: usize) -> Result<u32, String> {
  args
    .get(idx)
    .and_then(|d| match d.value {
      ResolvedValue::Integer(i) => Some(i as u32),
      _ => None,
    })
    .ok_or_else(|| "Could not parse".to_string())
}
