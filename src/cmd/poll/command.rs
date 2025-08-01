use super::actor::PollMessage;
use crate::cmd::{poll::pollstate::PollState, AppInteractor, CallContext};
use derive_new::new;
use kitchen_sink::actor::ActorHandle;
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

pub const NAME: &str = "poll";

#[derive(new)]
pub struct Poll {
  actor: ActorHandle<PollMessage>,
}

#[async_trait]
impl AppInteractor for Poll {
  #[instrument(name = NAME, level = "INFO", skip(self))]
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
          format!("option_{i}"),
          format!("Option to add to poll #{i}"),
        )
        .required(true),
      );
    }

    for i in 2..9 {
      command = command.add_option(
        CreateCommandOption::new(
          CommandOptionType::String,
          format!("option_{i}"),
          format!("Option to add to poll #{i}"),
        )
        .required(false),
      );
    }
    vec![command]
  }

  #[instrument(name = NAME, level = "INFO", skip(self, ctx, itx))]
  async fn app_interact(&self, ctx: &Context, itx: &CommandInteraction) {
    if !itx.data.name.as_str().eq(NAME) {
      return;
    }
    if let Err(e) = self._handle_app(ctx, itx).await {
      error!("Failed to create poll {:?}", e);
      let _ = itx
        .edit_response(
          &ctx.http,
          EditInteractionResponse::new().content(format!("{e}")),
        )
        .await;
    }
  }

  #[instrument(name = NAME, level = "INFO", skip(self, ctx, itx))]
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
    let poll_state = PollState::from_args(itx)?;
    let pm = PollMessage::CreatePoll(Box::new((
      poll_state,
      CallContext {
        http: ctx.http.clone(),
      },
    )));
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
      .send(PollMessage::UpdateVote(Box::new((
        poll_id,
        user,
        ctx.clone(),
        itx.clone(),
      ))))
      .await;
    Ok(())
  }
}
