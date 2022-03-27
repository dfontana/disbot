use std::error::Error;

use crate::{
  cmd::poll::{cache::Cache, pollstate::PollState},
  emoji::EmojiLookup,
};
use humantime::format_duration;
use once_cell::sync::Lazy;
use serenity::{
  builder::{CreateActionRow, CreateComponents},
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::{
    channel::{Message, ReactionType},
    guild::Emoji,
    interactions::message_component::MessageComponentInteraction,
  },
  utils::MessageBuilder,
};
use tracing::{error, instrument, warn};
use uuid::Uuid;

static POLL_STATES: Lazy<Cache<Uuid, PollState>> = Lazy::new(|| Cache::new());

#[command]
#[description = "Create a Poll with up to 9 Options. Double quote each argument."]
#[usage = "poll Duration Question voteItem1 voteItem2 [voteItem3] ... [voteItem9]"]
#[example = "\"1hour\" \"Whats the right pet?\" \"cats\" \"dogs\" will create a 2 option poll expiiring in 1 hour. Valid time units: 'day', 'hour', 'minute'"]
#[min_args(4)]
#[max_args(10)]
async fn poll(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
  let guild_id = match msg.guild_id {
    Some(id) => id,
    None => return Ok(()),
  };
  let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;

  // Create the poll
  let poll_state = PollState::from_args(args)?;

  // Send the poll message
  msg
    .channel_id
    .send_message(&ctx.http, |builder| {
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
      builder.content(poll_msg).set_components(component)
    })
    .await?;

  // Register globally
  let exp = poll_state.duration;
  let exp_key = poll_state.id;
  POLL_STATES.insert(poll_state.id, poll_state)?;

  // Setup the expiration action
  let exp_http = ctx.http.clone();
  let exp_chan = msg.channel_id;
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

pub struct PollHandler {}

impl PollHandler {
  pub fn new() -> Self {
    PollHandler {}
  }

  #[instrument(name = "Poller", level = "INFO", skip(self, ctx, itx))]
  pub async fn handle(&self, ctx: &Context, mut itx: MessageComponentInteraction) {
    if let Err(e) = self._handle(ctx, &mut itx).await {
      error!("Failed to update poll {:?}", e);
    }
  }

  async fn _handle(
    &self,
    ctx: &Context,
    itx: &mut MessageComponentInteraction,
  ) -> Result<(), Box<dyn Error>> {
    let poll_id = Uuid::parse_str(&itx.data.custom_id)?;
    if !POLL_STATES.contains_key(&poll_id)? {
      itx.defer(&ctx.http).await?;
      return Err("Skipping, poll has ended or not a poll interaction".into());
    }

    let user = get_user(&ctx, &itx).await;
    for value in itx.data.values.iter() {
      POLL_STATES.invoke_mut(&poll_id, |p| p.update_vote(value, &user))?;
    }

    let guild_id = match itx.guild_id {
      Some(g) => g,
      None => {
        itx.defer(&ctx.http).await?;
        return Err("No Guild Id on Interaction".into());
      }
    };
    let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;
    let new_body = POLL_STATES.invoke(&poll_id, |p| build_poll_message(&emoji, p))?;
    itx
      .message
      .edit(&ctx, |body| body.content(new_body))
      .await?;
    itx.defer(&ctx.http).await?;
    Ok(())
  }
}

async fn get_user(ctx: &Context, itx: &MessageComponentInteraction) -> String {
  if let Some(nick) = itx.user.nick_in(&ctx.http, itx.guild_id.unwrap()).await {
    return nick.to_lowercase();
  } else {
    return itx.user.name.to_lowercase();
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
