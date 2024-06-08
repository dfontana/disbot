use anyhow::anyhow;
use humantime::parse_duration;
use serenity::{
  all::CommandInteraction,
  http::Http,
  model::prelude::{ChannelId, Emoji},
  prelude::Context,
};
use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
  time::Duration,
};
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::cmd::{arg_util::Args, check_in::CheckInCtx};

use super::cache::Expiring;

#[derive(Clone)]
pub struct CallContext {
  pub channel: ChannelId,
  pub http: Arc<Http>,
  pub emoji: Emoji,
}

#[derive(Clone)]
pub struct PollState {
  pub id: Uuid,
  pub duration: Duration,
  pub topic: String,
  pub longest_option: usize,
  pub most_votes: usize,
  pub votes: HashMap<String, (String, usize, HashSet<String>)>,
  pub ctx: CallContext,
}

impl Expiring for PollState {
  fn duration(&self) -> Duration {
    self.duration
  }
}

impl From<CheckInCtx> for PollState {
  fn from(c: CheckInCtx) -> Self {
    PollState {
      id: Uuid::new_v4(),
      duration: c.poll_dur,
      topic: format!(
        "{}Will you be on tonight? This is a legally binding.",
        c.at_group.map(|c| format!("{} ", c)).unwrap_or("".into())
      ),
      longest_option: 3,
      most_votes: 0,
      votes: HashMap::from([
        ("1".into(), ("Yes".into(), 0, HashSet::new())),
        ("2".into(), ("No".into(), 0, HashSet::new())),
      ]),
      ctx: CallContext {
        channel: c.channel,
        http: c.http,
        emoji: c.emoji,
      },
    }
  }
}

impl PollState {
  pub fn from_args(
    ctx: &Context,
    emoji: Emoji,
    itx: &CommandInteraction,
  ) -> Result<PollState, anyhow::Error> {
    let raw_args = &itx.data.options();
    let args = Args::from(raw_args);

    let duration: Duration = args
      .str("duration")
      .map_err(|e| anyhow!("Duration not given").context(e))
      .and_then(|s| parse_duration(&s).map_err(|e| anyhow!("Invalid duration given").context(e)))?;

    let topic: String = args
      .str("topic")
      .map_err(|e| anyhow!("No topic given").context(e))
      .map(|s| s.to_string())?;

    let items: Vec<String> = { 0..9 }
      .map(|i| format!("option_{}", i))
      .filter_map(|key| args.str(&key).ok())
      .map(|s| s.to_string())
      .collect();

    if items.is_empty() {
      return Err(anyhow!("None or Malformed options given"));
    }

    let opt_width = items.iter().map(String::len).max().unwrap_or(1);
    let mut votes = HashMap::new();
    items.iter().enumerate().for_each(|(idx, it)| {
      votes.insert(format!("{}", idx + 1), (it.to_owned(), 0, HashSet::new()));
    });

    Ok(PollState {
      id: Uuid::new_v4(),
      duration,
      topic,
      longest_option: opt_width,
      most_votes: 0,
      votes,
      ctx: CallContext {
        channel: itx.channel_id,
        http: ctx.http.clone(),
        emoji,
      },
    })
  }

  #[instrument(name = "PollState", level = "INFO", skip(self))]
  pub fn update_vote(&mut self, votes: &[String], voter: &String) {
    info!("Casting vote");
    for (option, (_, count, voters)) in self.votes.iter_mut() {
      match (voters.contains(voter), votes.contains(option)) {
        (false, true) => {
          *count += 1;
          voters.insert(voter.into());
        }
        (true, false) => {
          *count -= 1;
          voters.remove(voter);
        }
        _ => (), // Already voted/not voted
      }
    }
    self.set_highest_vote();
  }

  fn set_highest_vote(&mut self) {
    self.most_votes = self.votes.values().map(|e| e.1).max().unwrap_or(0);
  }
}
