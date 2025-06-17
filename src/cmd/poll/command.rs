use super::actor::PollMessage;
use crate::{
  actor::ActorHandle,
  cmd::{poll::pollstate::PollState, AppInteractor},
  emoji::EmojiLookup,
};
use anyhow::anyhow;
use derive_new::new;
use serenity::{
  all::{CommandInteraction, CommandOptionType, CommandType, ComponentInteraction},
  async_trait,
  builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse,
  },
  client::Context,
};
use std::error::Error;
use tracing::{error, instrument, warn};
use uuid::Uuid;

const NAME: &str = "poll";

#[derive(new)]
pub struct Poll {
  emoji: EmojiLookup,
  actor: ActorHandle<PollMessage>,
}

#[async_trait]
impl AppInteractor for Poll {
  #[instrument(name = "Poller", level = "INFO", skip(self))]
  fn commands(&self) -> Vec<CreateCommand> {
    let mut command = CreateCommand::new(NAME)
      .description("Create a Poll with up to 9 Options")
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
          "topic",
          "Question or topic of the poll",
        )
        .required(true),
      );
    for i in 0..2 {
      command = command.add_option(
        CreateCommandOption::new(
          CommandOptionType::String,
          format!("option_{}", i),
          format!("Option to add to poll #{}", i),
        )
        .required(true),
      );
    }

    for i in 2..9 {
      command = command.add_option(
        CreateCommandOption::new(
          CommandOptionType::String,
          format!("option_{}", i),
          format!("Option to add to poll #{}", i),
        )
        .required(true),
      );
    }
    vec![command]
  }

  #[instrument(name = "Poller", level = "INFO", skip(self, ctx, itx))]
  async fn app_interact(&self, ctx: &Context, itx: &CommandInteraction) {
    if !itx.data.name.as_str().eq(NAME) {
      return;
    }
    if let Err(e) = self._handle_app(ctx, itx).await {
      error!("Failed to create poll {:?}", e);
      let _ = itx
        .edit_response(
          &ctx.http,
          EditInteractionResponse::new().content(format!("{}", e)),
        )
        .await;
    }
  }

  #[instrument(name = "Poller", level = "INFO", skip(self, ctx, itx))]
  async fn msg_interact(&self, ctx: &Context, itx: &ComponentInteraction) {
    let mut err = false;
    if let Err(e) = self._handle_msg(ctx, itx).await {
      error!("Failed to update poll {:?}", e);
      err = true;
    }
    if err {
      if let Err(e) = itx.defer(&ctx.http).await {
        error!("Failed to notify app failed {:?}", e);
      }
    }
  }
}

impl Poll {
  async fn _handle_app(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
  ) -> Result<(), anyhow::Error> {
    let guild_id = itx
      .guild_id
      .ok_or_else(|| anyhow!("No Guild Id on Interaction"))?;
    let pm = self
      .emoji
      .get(&ctx.http, &ctx.cache, guild_id)
      .await
      .and_then(|emoji| PollState::from_args(ctx, emoji, itx))
      .map(|ps| PollMessage::CreatePoll((ps, itx.channel_id)))?;
    self.actor.send(pm).await;
    let _ = itx
      .create_response(
        &ctx.http,
        CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("yep.")),
      )
      .await;
    Ok(())
  }

  async fn _handle_msg(
    &self,
    ctx: &Context,
    itx: &ComponentInteraction,
  ) -> Result<(), Box<dyn Error>> {
    let poll_id = Uuid::parse_str(&itx.data.custom_id)?;

    let user = if let Some(nick) = itx.user.nick_in(&ctx.http, itx.guild_id.unwrap()).await {
      nick.to_lowercase()
    } else {
      itx.user.name.to_lowercase()
    };

    self
      .actor
      .send(PollMessage::UpdateVote((
        poll_id,
        user,
        ctx.clone(),
        itx.clone(),
      )))
      .await;
    Ok(())
  }
}
