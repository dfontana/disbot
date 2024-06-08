use super::{CheckInCtx, CheckInMessage};
use crate::{
  actor::ActorHandle,
  cmd::{arg_util::Args, AppInteractor},
  emoji::EmojiLookup,
};
use anyhow::anyhow;
use chrono::NaiveTime;
use derive_new::new;
use humantime::parse_duration;
use serenity::{
  all::{CommandInteraction, CommandOptionType, CommandType, Role},
  async_trait,
  builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse,
  },
  prelude::Context,
  utils::MessageBuilder,
};
use std::time::Duration;
use tracing::{error, instrument};

const NAME: &str = "check-in";

#[derive(new)]
pub struct CheckIn {
  emoji: EmojiLookup,
  actor: ActorHandle<CheckInMessage>,
}

#[async_trait]
impl AppInteractor for CheckIn {
  #[instrument(name = "CheckIn", level = "INFO", skip(self))]
  fn commands(&self) -> Vec<CreateCommand> {
    vec![CreateCommand::new(NAME)
      .description("Create a Check In for this Channel")
      .kind(CommandType::ChatInput)
      .add_option(
        CreateCommandOption::new(
          CommandOptionType::String,
          "duration",
          "How long until poll closes. Valid time units: 'day', 'hour', 'minute'. ex: 30minute",
        )
        .required(true),
      )
      .add_option(
        CreateCommandOption::new(
          CommandOptionType::String,
          "time",
          "What time to run the poll, eg 19:30:00",
        )
        .required(true),
      )
      .add_option(
        CreateCommandOption::new(CommandOptionType::Role, "role", "What role to tag, if any")
          .required(false),
      )]
  }

  #[instrument(name = "CheckIn", level = "INFO", skip(self, ctx, itx))]
  async fn app_interact(&self, ctx: &Context, itx: &CommandInteraction) {
    if !itx.data.name.as_str().eq(NAME) {
      return;
    }
    if let Err(e) = self._handle_app(ctx, itx).await {
      error!("Failed to create poll {:?}", e);
      let _ = itx
        .edit_response(
          &ctx.http,
          EditInteractionResponse::new().content(&format!("{}", e)),
        )
        .await;
    }
  }
}

impl CheckIn {
  async fn _handle_app(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
  ) -> Result<(), anyhow::Error> {
    let guild_id = itx
      .guild_id
      .ok_or_else(|| anyhow!("No Guild Id on Interaction"))?;
    let emoji = self.emoji.get(&ctx.http, &ctx.cache, guild_id).await?;
    let raw_args = &itx.data.options();
    let args = Args::from(raw_args);

    let duration: Duration = args
      .str("duration")
      .map_err(|e| anyhow!("Duration not given").context(e))
      .and_then(|s| parse_duration(&s).map_err(|e| anyhow!("Invalid duration given").context(e)))?;

    let time: NaiveTime = args
      .str("time")
      .map_err(|e| anyhow!("No time given").context(e))
      .and_then(|s| {
        s.parse::<NaiveTime>()
          .map_err(|e| anyhow!("Invalid time given").context(e))
      })?;

    let at_group: Option<Role> = args
      .opt_role("role")
      .map_err(|e| anyhow!("Invalid role given").context(e))
      .map(|v| v.map(|r| r.clone()))?;

    self
      .actor
      .send(CheckInMessage::SetPoll(CheckInCtx::new(
        time,
        duration,
        at_group,
        itx.channel_id,
        ctx.http.clone(),
        emoji.clone(),
      )))
      .await;
    itx
      .create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
          CreateInteractionResponseMessage::new().content(
            MessageBuilder::new()
              .emoji(&emoji)
              .push_bold("Check in set to ")
              .push_italic(time.to_string())
              .push_bold(" lasting ")
              .push_italic(duration.as_secs().to_string())
              .push_bold(" seconds.")
              .emoji(&emoji)
              .build(),
          ),
        ),
      )
      .await?;
    Ok(())
  }
}
