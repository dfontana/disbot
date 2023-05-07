use std::{collections::HashMap, error::Error, time::Duration};

use chrono::NaiveTime;
use derive_new::new;
use humantime::parse_duration;
use serenity::{
  async_trait,
  builder::CreateApplicationCommands,
  model::prelude::{
    command::{CommandOptionType, CommandType},
    interaction::{
      application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
      InteractionResponseType,
    },
  },
  prelude::Context,
  utils::MessageBuilder,
};
use tracing::{error, instrument};

use crate::{actor::ActorHandle, cmd::AppInteractor, emoji::EmojiLookup};

use super::{CheckInCtx, CheckInMessage};

const NAME: &str = "check-in";

#[derive(new)]
pub struct CheckIn {
  emoji: EmojiLookup,
  actor: ActorHandle<CheckInMessage>,
}

#[async_trait]
impl AppInteractor for CheckIn {
  #[instrument(name = "CheckIn", level = "INFO", skip(self, commands))]
  fn register(&self, commands: &mut CreateApplicationCommands) {
    commands.create_application_command(|command| {
      command
        .name(NAME)
        .description("Create a Check In for this Channel")
        .kind(CommandType::ChatInput)
        .create_option(|option| {
          option
            .kind(CommandOptionType::String)
            .name("duration")
            .description(
              "How long until poll closes. Valid time units: 'day', 'hour', 'minute'. ex: 30minute",
            )
            .required(true)
        })
        .create_option(|option| {
          option
            .kind(CommandOptionType::String)
            .name("time")
            .description("What time to run the poll, eg 19:30:00")
            .required(true)
        })
    });
  }

  #[instrument(name = "CheckIn", level = "INFO", skip(self, ctx, itx))]
  async fn app_interact(&self, ctx: &Context, itx: &ApplicationCommandInteraction) {
    let mut err = false;
    if let Err(e) = self._handle_app(ctx, itx).await {
      error!("Failed to create poll {:?}", e);
      err = true;
    }
    if err {
      if let Err(e) = itx
        .create_interaction_response(&ctx.http, |bld| {
          bld
            .kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|f| f.content("Command failed"))
        })
        .await
      {
        error!("Failed to notify app failed {:?}", e);
      }
    }
  }
}

impl CheckIn {
  async fn _handle_app(
    &self,
    ctx: &Context,
    itx: &ApplicationCommandInteraction,
  ) -> Result<(), Box<dyn Error>> {
    if !itx.data.name.as_str().eq(NAME) {
      return Ok(());
    }
    let guild_id = match itx.guild_id {
      Some(g) => g,
      None => {
        return Err("No Guild Id on Interaction".into());
      }
    };
    let emoji = self.emoji.get(&ctx.http, &ctx.cache, guild_id).await?;
    let args = &itx.data.options;
    let map: HashMap<String, _> = args
      .iter()
      .map(|d| (d.name.to_owned(), d.resolved.to_owned()))
      .collect();

    let duration: Duration = map
      .get("duration")
      .and_then(|v| v.to_owned())
      .and_then(|d| match d {
        CommandDataOptionValue::String(v) => Some(v),
        _ => None,
      })
      .ok_or("No duration given")
      .and_then(|s| parse_duration(&s).map_err(|_| "Invalid duration given"))?;

    let time: NaiveTime = map
      .get("time")
      .and_then(|v| v.to_owned())
      .and_then(|d| match d {
        CommandDataOptionValue::String(v) => Some(v),
        _ => None,
      })
      .ok_or("No time given")
      .and_then(|s| s.parse::<NaiveTime>().map_err(|_| "Invalid time given"))?;

    self
      .actor
      .send(CheckInMessage::SetPoll((
        time,
        CheckInCtx::new(duration, itx.channel_id, ctx.http.clone(), emoji.clone()),
      )))
      .await;
    itx
      .create_interaction_response(&ctx.http, |f| {
        f.kind(InteractionResponseType::ChannelMessageWithSource)
          .interaction_response_data(|k| {
            k.content(
              MessageBuilder::new()
                .mention(&emoji)
                .push_bold("Check in set to ")
                .push_italic(time)
                .push_bold(" lasting ")
                .push_italic(duration.as_secs())
                .push_bold(" seconds.")
                .mention(&emoji)
                .build(),
            )
          })
      })
      .await?;
    Ok(())
  }
}
