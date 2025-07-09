use super::cache::Expiring;
use crate::cmd::poll::NAME;
use crate::cmd::{arg_util::Args, check_in::CheckInCtx};
use crate::persistence::Expirable;
use crate::types::{Chan, Guil, Pid};
use anyhow::anyhow;
use bincode::{Decode, Encode};
use humantime::parse_duration;
use serenity::all::CommandInteraction;
use std::{
  collections::{HashMap, HashSet},
  time::{Duration, SystemTime},
};
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Clone, Encode, Decode)]
pub struct PollState {
  pub id: Pid,
  pub duration: Duration,
  pub topic: String,
  pub longest_option: usize,
  pub most_votes: usize,
  pub votes: HashMap<String, (String, usize, HashSet<String>)>,
  pub created_at: SystemTime,
  pub channel: Chan,
  pub guild: Guil,
}

impl Expiring for PollState {
  fn duration(&self) -> Duration {
    self.duration
  }
}

impl Expirable for PollState {
  fn is_expired(&self) -> bool {
    self.elapsed() >= self.duration
  }
}

impl From<CheckInCtx> for PollState {
  fn from(c: CheckInCtx) -> Self {
    PollState {
      id: Pid(Uuid::new_v4()),
      duration: c.poll_dur,
      topic: format!(
        "{}Will you be on tonight? This is a legally binding.",
        c.at_group.map(|c| format!("{} ", *c)).unwrap_or("".into())
      ),
      longest_option: 3,
      most_votes: 0,
      votes: HashMap::from([
        ("1".into(), ("Yes".into(), 0, HashSet::new())),
        ("2".into(), ("No".into(), 0, HashSet::new())),
      ]),
      created_at: SystemTime::now(),
      channel: c.channel,
      guild: c.guild,
    }
  }
}

impl PollState {
  pub fn from_args(itx: &CommandInteraction) -> Result<PollState, anyhow::Error> {
    let raw_args = &itx.data.options();
    let args = Args::from(raw_args);

    let guild = itx
      .guild_id
      .ok_or_else(|| anyhow!("No Guild Id on Interaction"))?;

    let duration: Duration = args
      .str("duration")
      .map_err(|e| anyhow!("Duration not given").context(e))
      .and_then(|s| parse_duration(s).map_err(|e| anyhow!("Invalid duration given").context(e)))?;

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
      id: Pid(Uuid::new_v4()),
      duration,
      topic,
      longest_option: opt_width,
      most_votes: 0,
      votes,
      created_at: SystemTime::now(),
      channel: Chan(itx.channel_id),
      guild: Guil(guild),
    })
  }

  #[instrument(name = NAME, level = "INFO", skip(self))]
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

  pub fn elapsed(&self) -> Duration {
    SystemTime::now()
      .duration_since(self.created_at)
      // Assume full duration elapsed if now is before created_at
      .unwrap_or(self.duration)
  }
}
