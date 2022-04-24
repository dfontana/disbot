use std::error::Error;

use super::AppInteractor;
use crate::emoji::EmojiLookup;
use derive_new::new;
use rand::Rng;
use serenity::{
  async_trait,
  builder::CreateApplicationCommands,
  client::Context,
  model::interactions::{
    application_command::{
      ApplicationCommandInteraction, ApplicationCommandInteractionDataOption,
      ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType,
      ApplicationCommandType,
    },
    InteractionResponseType,
  },
  utils::MessageBuilder,
};
use tracing::{instrument, log::error};

const NAME: &'static str = "roll";

#[derive(new)]
pub struct DiceRoll {
  emoji: EmojiLookup,
}

#[async_trait]
impl AppInteractor for DiceRoll {
  #[instrument(name = "Roll", level = "INFO", skip(self, commands))]
  fn register(&self, commands: &mut CreateApplicationCommands) {
    commands.create_application_command(|command| {
      command
        .name(NAME)
        .description("Roll a die, optionally between the given bounds")
        .kind(ApplicationCommandType::ChatInput)
        .create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::Integer)
            .name("lower")
            .description("Above or equal to")
            .min_int_value(0)
            .max_int_value(100)
            .required(false)
        })
        .create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::Integer)
            .name("upper")
            .description("Below or equal to")
            .min_int_value(0)
            .max_int_value(100)
            .required(false)
        })
    });
  }

  #[instrument(name = "Roll", level = "INFO", skip(self, ctx, itx))]
  async fn app_interact(&self, ctx: &Context, itx: &ApplicationCommandInteraction) {
    let mut err = false;
    if let Err(e) = self._handle_app(ctx, itx).await {
      error!("Failed to roll {:?}", e);
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

impl DiceRoll {
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
          .interaction_response_data(|f| f.content("Rolling..."))
      })
      .await?;

    let (lower, upper) = match validate(&itx.data.options) {
      Ok(v) => v,
      Err(e) => {
        itx
          .edit_original_interaction_response(&ctx.http, |f| f.content(e))
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
      .mention(&emoji)
      .push(" ");

    match roll {
      1 => response.mention(&emoji),
      21 => response.push_bold("21 - you stupid!"),
      47 => response.push_bold("god damn 47"),
      69 => response.push_bold("69").push_italic("...nice"),
      _ => response.push_bold(roll),
    };

    let resp_string = response
      .push(" ")
      .mention(&emoji)
      .push(" ")
      .push_mono(format!("({} - {})", lower, upper))
      .build();

    itx
      .edit_original_interaction_response(&ctx.http, |f| f.content(resp_string))
      .await?;

    Ok(())
  }
}

fn validate(args: &Vec<ApplicationCommandInteractionDataOption>) -> Result<(u32, u32), String> {
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

fn extract_int(
  args: &Vec<ApplicationCommandInteractionDataOption>,
  idx: usize,
) -> Result<u32, String> {
  args
    .get(idx)
    .map(|d| match d.resolved {
      Some(ApplicationCommandInteractionDataOptionValue::Integer(i)) => Some(i as u32),
      _ => None,
    })
    .flatten()
    .ok_or("Could not parse".to_string())
}
