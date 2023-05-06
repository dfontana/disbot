use std::error::Error;

use crate::{
  cmd::{poll::pollstate::PollState, AppInteractor},
  emoji::EmojiLookup,
};
use derive_new::new;

use serenity::{
  async_trait,
  builder::CreateApplicationCommands,
  client::Context,
  model::prelude::{
    command::{CommandOptionType, CommandType},
    interaction::{
      application_command::ApplicationCommandInteraction,
      message_component::MessageComponentInteraction, InteractionResponseType,
    },
  },
};
use tracing::{error, instrument, warn};
use uuid::Uuid;

use super::actor::{PollHandle, PollMessage};

const NAME: &str = "poll";

#[derive(new)]
pub struct Poll {
  emoji: EmojiLookup,
  actor: PollHandle,
}

#[async_trait]
impl AppInteractor for Poll {
  #[instrument(name = "Poller", level = "INFO", skip(self, commands))]
  fn register(&self, commands: &mut CreateApplicationCommands) {
    commands.create_application_command(|command| {
      command
        .name(NAME)
        .description("Create a Poll with up to 9 Options")
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
            .name("topic")
            .description("Question or topic of the poll")
            .required(true)
        });

      for i in 0..2 {
        command.create_option(|option| {
          option
            .kind(CommandOptionType::String)
            .name(format!("option_{}", i))
            .description(format!("Option to add to poll #{}", i))
            .required(true)
        });
      }

      for i in 2..9 {
        command.create_option(|option| {
          option
            .kind(CommandOptionType::String)
            .name(format!("option_{}", i))
            .description(format!("Option to add to poll #{}", i))
            .required(false)
        });
      }
      command
    });
  }

  #[instrument(name = "Poller", level = "INFO", skip(self, ctx, itx))]
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

  #[instrument(name = "Poller", level = "INFO", skip(self, ctx, itx))]
  async fn msg_interact(&self, ctx: &Context, itx: &MessageComponentInteraction) {
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
    let ps = PollState::from_args(ctx, emoji, itx)?;
    self
      .actor
      .send(PollMessage::CreatePoll((ps, itx.clone())))
      .await;
    Ok(())
  }

  async fn _handle_msg(
    &self,
    ctx: &Context,
    itx: &MessageComponentInteraction,
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
