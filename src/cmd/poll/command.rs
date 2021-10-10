use std::error::Error;

use crate::{
  cmd::poll::{cache::Cache, pollstate::PollState},
  emoji::EmojiLookup,
};
use humantime::format_duration;
use serenity::{
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::{
    channel::{Message, Reaction, ReactionType},
    guild::Emoji,
    id::MessageId,
  },
  utils::MessageBuilder,
};
use tracing::{error, instrument, warn};

lazy_static! {
  static ref POLL_STATES: Cache<MessageId, PollState> = Cache::new();
}

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
  let res_msg = msg
    .reply(&ctx.http, build_poll_message(&emoji, &poll_state))
    .await?;

  // Add reactions for users to vote
  let number_emoji = EmojiLookup::inst().get_numbers();
  let emotes: Vec<ReactionType> = (1..poll_state.votes.len() + 1)
    .map(|i| number_emoji.get(&i))
    .filter(Option::is_some)
    .map(|em| ReactionType::Unicode(em.unwrap().to_owned()))
    .collect();
  for emote in emotes {
    res_msg.react(&ctx.http, emote).await?;
  }

  // Register globally
  let exp = poll_state.duration.clone();
  POLL_STATES.insert(res_msg.id, poll_state)?;

  // Setup the expiration action
  let exp_http = ctx.http.clone();
  let exp_chan = msg.channel_id.clone();
  let exp_key = res_msg.id.clone();
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

  async fn _update_poll<F>(
    &self,
    ctx: &Context,
    react: &Reaction,
    apply: F,
  ) -> Result<(), Box<dyn Error>>
  where
    F: FnOnce(&MessageId, usize) -> Result<(), String>,
  {
    if react.user_id.unwrap() == ctx.cache.as_ref().current_user().await.id {
      return Err("Skipping, self reaction".into());
    }
    if !POLL_STATES.contains_key(&react.message_id)? {
      return Err("Skipping, not a poll reaction".into());
    }

    let vote = EmojiLookup::inst().to_number(&react.emoji);
    if vote.is_none() {
      return Err("Skipping, not a valid poll emote".into());
    }

    apply(&react.message_id, vote.unwrap())?;

    let guild_id = match react.guild_id {
      Some(g) => g,
      None => return Err("No Guild Id on Reaction".into()),
    };
    let mut msg = react.message(&ctx.http).await?;
    let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;
    let new_body = POLL_STATES.invoke(&react.message_id, |p| build_poll_message(&emoji, p))?;
    msg.edit(&ctx, |body| body.content(new_body)).await?;

    Ok(())
  }

  #[instrument(name = "Poller", level = "INFO", skip(self, ctx, react))]
  pub async fn add_vote(&self, ctx: &Context, react: &Reaction) {
    let update_op =
      |msgid: &MessageId, vote: usize| POLL_STATES.invoke_mut(msgid, |p| p.cast_vote(vote));
    match self._update_poll(ctx, react, update_op).await {
      Err(e) => {
        error!("Failed to add vote {:?}", e);
        return;
      }
      _ => (),
    }
  }

  #[instrument(name = "Poller", level = "INFO", skip(self, ctx, react))]
  pub async fn remove_vote(&self, ctx: &Context, react: &Reaction) {
    let update_op =
      |msgid: &MessageId, vote: usize| POLL_STATES.invoke_mut(msgid, |p| p.revoke_vote(vote));
    match self._update_poll(ctx, react, update_op).await {
      Err(e) => {
        error!("Failed to remove vote {:?}", e);
        return;
      }
      _ => (),
    }
  }
}

fn build_poll_message(emoji: &Emoji, poll_state: &PollState) -> String {
  let mut bar_vec = poll_state
    .votes
    .iter()
    .map(|(idx, (opt, votes))| {
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

  MessageBuilder::new()
    .mention(emoji)
    .push_underline("Roommate Poll, Bobby, Roommate Poll!")
    .mention(emoji)
    .push_line("")
    .push_line("")
    .push_bold(&poll_state.topic)
    .push_italic(format!(" (exp in {})", format_duration(poll_state.duration)))
    .push_line("")
    .push_codeblock(&bar_vec.join("\n"), Some("m"))
    .build()
}

fn build_exp_message(emoji: &Emoji, poll_state: &PollState) -> String {
  let winner = poll_state
    .votes
    .values()
    .max_by(|a, b| a.1.cmp(&b.1))
    .map(|v| v.0.to_string())
    .unwrap_or("No one!".to_string());

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
