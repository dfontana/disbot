use std::error::Error;

use crate::{
  cmd::{
    poll::{cache::Cache, pollstate::PollState},
    AppInteractor,
  },
  emoji::EmojiLookup,
};
use humantime::format_duration;
use once_cell::sync::Lazy;
use serenity::{
  async_trait,
  builder::{CreateActionRow, CreateApplicationCommands, CreateComponents},
  client::Context,
  model::{
    channel::ReactionType,
    guild::Emoji,
    interactions::{
      application_command::{
        ApplicationCommandInteraction, ApplicationCommandOptionType, ApplicationCommandType,
      },
      message_component::MessageComponentInteraction,
      InteractionResponseType,
    },
  },
  utils::MessageBuilder,
};
use tracing::{error, instrument, warn};
use uuid::Uuid;

const NAME: &'static str = "poll";
static POLL_STATES: Lazy<Cache<Uuid, PollState>> = Lazy::new(|| Cache::new());

#[derive(Default)]
pub struct Poll {}

#[async_trait]
impl AppInteractor for Poll {
  #[instrument(name = "Poller", level = "INFO", skip(self, commands))]
  fn register(&self, commands: &mut CreateApplicationCommands) {
    commands.create_application_command(|command| {
      command
        .name(NAME)
        .description("Create a Poll with up to 9 Options")
        .kind(ApplicationCommandType::ChatInput)
        .create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::String)
            .name("duration")
            .description(
              "How long until poll closes. Valid time units: 'day', 'hour', 'minute'. ex: 30minute",
            )
            .required(true)
        })
        .create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::String)
            .name("topic")
            .description("Question or topic of the poll")
            .required(true)
        });

      for i in 0..2 {
        command.create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::String)
            .name(format!("option_{}", i))
            .description(format!("Option to add to poll #{}", i))
            .required(true)
        });
      }

      for i in 2..9 {
        command.create_option(|option| {
          option
            .kind(ApplicationCommandOptionType::String)
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
    if let Err(e) = self._handle(ctx, itx).await {
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
    let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;

    // Create the poll
    let poll_state = PollState::from_args(&itx.data.options)?;

    // Send the poll message
    itx
      .create_interaction_response(&ctx.http, |builder| {
        let poll_msg = build_poll_message(&emoji, &poll_state);
        let mut component = CreateComponents::default();
        let mut action_row = CreateActionRow::default();

        action_row.create_select_menu(|select| {
          select
            .placeholder("Choose your Answers")
            .custom_id(poll_state.id)
            .min_values(1)
            .max_values(poll_state.votes.len() as u64)
            .options(|opts| {
              poll_state.votes.iter().for_each(|(k, v)| {
                opts.create_option(|opt| {
                  opt
                    .label(v.0.to_owned())
                    .value(k.to_owned())
                    .emoji(ReactionType::Custom {
                      name: None,
                      animated: false,
                      id: emoji.id,
                    })
                });
              });
              opts
            })
        });

        component.add_action_row(action_row);

        builder
          .kind(InteractionResponseType::ChannelMessageWithSource)
          .interaction_response_data(|f| f.content(poll_msg).set_components(component))
      })
      .await?;

    // Register globally
    let exp = poll_state.duration;
    let exp_key = poll_state.id;
    POLL_STATES.insert(poll_state.id, poll_state)?;

    // Setup the expiration action
    let exp_http = ctx.http.clone();
    let exp_chan = itx.channel_id;
    let exp_emote = emoji.clone();
    tokio::spawn(async move {
      tokio::time::sleep(exp).await;
      let resp = match POLL_STATES.invoke(&exp_key, |p| build_exp_message(&exp_emote, p)) {
        Err(_) => "Poll has ended -- failed to get details".to_string(),
        Ok(v) => v,
      };
      let _ = exp_chan.say(&exp_http, resp).await;
      if let Err(e) = POLL_STATES.remove(&exp_key) {
        warn!("Failed to reap poll on exp: {}", e);
      }
    });

    Ok(())
  }

  async fn _handle(
    &self,
    ctx: &Context,
    itx: &MessageComponentInteraction,
  ) -> Result<(), Box<dyn Error>> {
    let poll_id = Uuid::parse_str(&itx.data.custom_id)?;
    if !POLL_STATES.contains_key(&poll_id)? {
      return Err("Skipping, poll has ended or not a poll interaction".into());
    }

    let user = if let Some(nick) = itx.user.nick_in(&ctx.http, itx.guild_id.unwrap()).await {
      nick.to_lowercase()
    } else {
      itx.user.name.to_lowercase()
    };

    for value in itx.data.values.iter() {
      POLL_STATES.invoke_mut(&poll_id, |p| p.update_vote(value, &user))?;
    }

    let guild_id = match itx.guild_id {
      Some(g) => g,
      None => return Err("No Guild Id on Interaction".into()),
    };
    let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;
    let new_body = POLL_STATES.invoke(&poll_id, |p| build_poll_message(&emoji, p))?;
    itx
      .message
      .clone()
      .edit(&ctx, |body| body.content(new_body))
      .await?;
    itx.defer(&ctx.http).await?;
    Ok(())
  }
}

fn build_poll_message(emoji: &Emoji, poll_state: &PollState) -> String {
  let mut bar_vec = poll_state
    .votes
    .iter()
    .map(|(idx, (opt, votes, _))| {
      format!(
        "{}: {:<opt_width$} | {:#<votes$}{:<bar_width$} | ({})",
        idx,
        opt,
        "",
        "",
        votes,
        votes = votes,
        opt_width = poll_state.longest_option,
        bar_width = poll_state.most_votes - votes
      )
    })
    .collect::<Vec<String>>();
  bar_vec.sort();

  let mut voter_vec = poll_state
    .votes
    .iter()
    .map(|(idx, (_, _, voters))| {
      format!(
        "{}: {}",
        idx,
        voters
          .iter()
          .map(|v| v.to_string())
          .collect::<Vec<String>>()
          .join(", ")
      )
    })
    .collect::<Vec<String>>();
  voter_vec.sort();

  MessageBuilder::new()
    .mention(emoji)
    .push_underline("Roommate Poll, Bobby, Roommate Poll!")
    .mention(emoji)
    .push_line("")
    .push_line("")
    .push_bold(&poll_state.topic)
    .push_italic(format!(
      " (exp in {})",
      format_duration(poll_state.duration)
    ))
    .push_line("")
    .push_codeblock(
      format!(
        "{}\n\nVoters:\n{}",
        &bar_vec.join("\n"),
        voter_vec.join("\n")
      ),
      Some("m"),
    )
    .build()
}

fn build_exp_message(emoji: &Emoji, poll_state: &PollState) -> String {
  let winner = poll_state
    .votes
    .values()
    .max_by(|a, b| a.1.cmp(&b.1))
    .map(|v| v.0.to_string())
    .unwrap_or_else(|| "<Error Poll Had No Options?>".to_string());

  MessageBuilder::new()
    .mention(emoji)
    .push_underline("The Vote has Ended!")
    .mention(emoji)
    .push_line("")
    .push_line("")
    .push("The winner of \"")
    .push_bold(&poll_state.topic)
    .push("\" is: ")
    .push_bold(&winner)
    .push_line("")
    .push_italic("(Ties are resolved by the righteous power vested in me - deal with it)")
    .build()
}
