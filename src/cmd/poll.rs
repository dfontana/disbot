use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
};

use crate::{debug::Debug, emoji::EmojiLookup};
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

lazy_static! {
  // TODO have these fall out of memory after 24hrs
  static ref POLL_STATES: Arc<RwLock<HashMap<MessageId, PollState>>> = Arc::new(RwLock::new(HashMap::new()));
}

// TODO
//   Track who voted for what & list their names next to the vote
//   Allow polls only 1 vote per user (optional)
struct PollState {
  topic: String,
  longest_option: usize,
  most_votes: usize,
  votes: HashMap<usize, (String, usize)>,
}

impl PollState {
  fn set_highest_vote(&mut self) {
    self.most_votes = self.votes.values().map(|e| e.1).max().unwrap_or(0);
  }
  fn cast_vote(&mut self, vote: usize) {
    if !self.votes.contains_key(&vote) {
      Debug::inst("poller").log("Vote not present in poll, ignoring");
      return;
    }
    self.votes.entry(vote).and_modify(|e| e.1 += 1);
    self.set_highest_vote();
  }

  fn revoke_vote(&mut self, vote: usize) {
    if !self.votes.contains_key(&vote) {
      Debug::inst("poller").log("Vote not present in poll, ignoring");
      return;
    }
    self.votes.entry(vote).and_modify(|e| e.1 -= 1);
    self.set_highest_vote();
  }
}

#[command]
#[description = "Create a Poll with up to 9 Options. Double quote each argument."]
#[usage = "pollQuestion voteItem1 voteItem2 [voteItem3] ... [voteItem9]"]
#[example = "\"Whats the right pet?\" \"cats\" \"dogs\" will create a 2 option poll"]
#[min_args(3)]
#[max_args(10)]
async fn poll(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
  let guild_id = match msg.guild_id {
    Some(id) => id,
    None => return Ok(()),
  };
  let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;

  // Create the poll
  let poll_state = build_poll(args)?;

  // Send the poll message
  let res = build_poll_body(&emoji, &poll_state)?;
  let res_msg = msg.reply(&ctx.http, res).await?;

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
  match POLL_STATES.write() {
    Ok(mut map) => {
      map.insert(res_msg.id, poll_state);
    }
    Err(_) => return Err("Failed to aquire state lock".into()),
  }

  Ok(())
}

pub struct PollHandler {}

impl PollHandler {
  pub fn new() -> Self {
    PollHandler {}
  }

  pub async fn add_vote(&self, ctx: &Context, react: &Reaction) {
    // TODO can this be simplified? Method is fugly.
    if react.user_id.unwrap() == ctx.cache.as_ref().current_user().await.id {
      Debug::inst("poller").log("Skipping, self reaction");
      return;
    }
    if !does_poll_exist(&react.message_id) {
      Debug::inst("poller").log("Skipping, not a poll reaction");
      return;
    }
    let vote = EmojiLookup::inst().to_number(&react.emoji);
    if vote.is_none() {
      Debug::inst("poller").log("Skipping, not a valid poll emote");
      return;
    }
    {
      // Wrapped in block to drop lock stat.
      match POLL_STATES.write() {
        Err(_) => {
          Debug::inst("poller").log("Failed to cast vote, lock not aquired");
          return;
        }
        Ok(mut polls) => {
          polls
            .get_mut(&react.message_id)
            .unwrap()
            .cast_vote(vote.unwrap());
        }
      }
    }
    let mut msg = match react.message(&ctx.http).await {
      Err(e) => {
        Debug::inst("poller").log(&format!("Failed to update poll message - {}", e));
        return;
      }
      Ok(msg) => msg,
    };
    let guild_id = match react.guild_id {
      Some(g) => g,
      None => {
        Debug::inst("poller").log("No Guild Id on Reaction");
        return;
      }
    };
    let emoji = match EmojiLookup::inst().get(guild_id, &ctx.cache).await {
      Err(e) => {
        Debug::inst("poller").log(&format!("Failed to get emoji for body - {}", e));
        return;
      }
      Ok(e) => e,
    };

    let new_body = match POLL_STATES.read() {
      Err(e) => {
        Debug::inst("poller").log(&format!("Failed to get poll for update - {}", e));
        return;
      }
      Ok(lock) => match build_poll_body(&emoji, lock.get(&react.message_id).unwrap()) {
        Ok(v) => v,
        Err(e) => {
          Debug::inst("poller").log(&format!("Failed to build poll body - {}", e));
          return;
        }
      },
    };
    match msg.edit(&ctx, |body| body.content(new_body)).await {
      Err(e) => {
        Debug::inst("poller").log(&format!("Failed to edit poll message - {}", e));
        return;
      }
      _ => (),
    }
  }

  pub async fn remove_vote(&self, ctx: &Context, react: &Reaction) {
    if react.user_id.unwrap() == ctx.cache.as_ref().current_user().await.id {
      Debug::inst("poller").log("Skipping, self reaction");
      return;
    }
    if !does_poll_exist(&react.message_id) {
      Debug::inst("poller").log("Skipping, not a poll reaction");
      return;
    }
    let vote = EmojiLookup::inst().to_number(&react.emoji);
    if vote.is_none() {
      Debug::inst("poller").log("Skipping, not a valid poll emote");
      return;
    }
    {
      // Wrapped in block to drop lock stat.
      match POLL_STATES.write() {
        Err(_) => {
          Debug::inst("poller").log("Failed to cast vote, lock not aquired");
          return;
        }
        Ok(mut polls) => {
          polls
            .get_mut(&react.message_id)
            .unwrap()
            .revoke_vote(vote.unwrap());
        }
      }
    }
    let mut msg = match react.message(&ctx.http).await {
      Err(e) => {
        Debug::inst("poller").log(&format!("Failed to update poll message - {}", e));
        return;
      }
      Ok(msg) => msg,
    };
    let guild_id = match react.guild_id {
      Some(g) => g,
      None => {
        Debug::inst("poller").log("No Guild Id on Reaction");
        return;
      }
    };
    let emoji = match EmojiLookup::inst().get(guild_id, &ctx.cache).await {
      Err(e) => {
        Debug::inst("poller").log(&format!("Failed to get emoji for body - {}", e));
        return;
      }
      Ok(e) => e,
    };

    let new_body = match POLL_STATES.read() {
      Err(e) => {
        Debug::inst("poller").log(&format!("Failed to get poll for update - {}", e));
        return;
      }
      Ok(lock) => match build_poll_body(&emoji, lock.get(&react.message_id).unwrap()) {
        Ok(v) => v,
        Err(e) => {
          Debug::inst("poller").log(&format!("Failed to build poll body - {}", e));
          return;
        }
      },
    };
    match msg.edit(&ctx, |body| body.content(new_body)).await {
      Err(e) => {
        Debug::inst("poller").log(&format!("Failed to edit poll message - {}", e));
        return;
      }
      _ => (),
    }
  }
}

fn does_poll_exist(id: &MessageId) -> bool {
  match POLL_STATES.read() {
    Err(_) => false,
    Ok(poll) => poll.contains_key(id),
  }
}

fn build_poll(mut args: Args) -> Result<PollState, String> {
  let topic = args.single_quoted::<String>().unwrap();
  let items: Vec<String> = args
    .trimmed()
    .quoted()
    .iter::<String>()
    .map(|arg| arg.unwrap().trim_matches('"').to_owned())
    .collect();
  if items.len() > 10 {
    Debug::inst("poll").log("Skipping poll, too many args");
    return Err("Too many arguments given".to_string());
  }

  let opt_width = items.iter().map(String::len).max().unwrap_or(1);
  let mut votes = HashMap::new();
  items.iter().enumerate().for_each(|(idx, it)| {
    votes.insert(idx + 1, (it.to_owned(), 0));
  });

  Ok(PollState {
    topic,
    longest_option: opt_width,
    most_votes: 0,
    votes,
  })
}

fn build_poll_body(emoji: &Emoji, poll_state: &PollState) -> Result<String, String> {
  let bars = poll_state
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
    .collect::<Vec<String>>()
    .join("\n");

  let mut response = MessageBuilder::new();
  let res = response
    .mention(emoji)
    .push_underline("Roommate Poll, Bobby, Roommate Poll!")
    .mention(emoji)
    .push_line("")
    .push_line("")
    .push_bold_line(&poll_state.topic)
    .push_codeblock(&bars, Some("m"))
    .build();
  Ok(res)
}
