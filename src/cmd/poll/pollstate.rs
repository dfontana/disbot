use humantime::parse_duration;
use serenity::model::interactions::application_command::{
  ApplicationCommandInteractionDataOption, ApplicationCommandInteractionDataOptionValue,
};
use std::{
  collections::{HashMap, HashSet},
  time::Duration,
};
use tracing::{info, instrument};
use uuid::Uuid;

use super::cache::Expiring;

pub struct PollState {
  pub id: Uuid,
  pub duration: Duration,
  pub topic: String,
  pub longest_option: usize,
  pub most_votes: usize,
  pub votes: HashMap<String, (String, usize, HashSet<String>)>,
}

impl Expiring for PollState {
  fn duration(&self) -> Duration {
    self.duration
  }
}

impl PollState {
  pub fn from_args(
    args: &Vec<ApplicationCommandInteractionDataOption>,
  ) -> Result<PollState, String> {
    let map: HashMap<String, _> = args
      .iter()
      .map(|d| (d.name.to_owned(), d.resolved.to_owned()))
      .collect();

    let duration: Duration = map
      .get("duration")
      .map(|v| v.to_owned())
      .flatten()
      .and_then(|d| match d {
        ApplicationCommandInteractionDataOptionValue::String(v) => Some(v),
        _ => None,
      })
      .ok_or("No duration given")
      .and_then(|s| parse_duration(&s).map_err(|_| "Invalid duration given"))?;

    let topic: String = map
      .get("topic")
      .map(|v| v.to_owned())
      .flatten()
      .and_then(|d| match d {
        ApplicationCommandInteractionDataOptionValue::String(v) => Some(v),
        _ => None,
      })
      .ok_or("No topic given")?;

    let items: Vec<String> = { 0..9 }
      .into_iter()
      .map(|i| format!("option_{}", i))
      .map(|key| map.get(&key))
      .flatten()
      .filter_map(|d| match d {
        Some(ApplicationCommandInteractionDataOptionValue::String(v)) => Some(v.to_owned()),
        _ => None,
      })
      .collect();

    if items.is_empty() {
      return Err("None or Malformed options given".into());
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
    })
  }

  #[instrument(name = "PollState", level = "INFO", skip(self))]
  pub fn update_vote(&mut self, vote: &String, voter: &String) {
    if !self.votes.contains_key(vote) {
      info!("Vote not present in poll, ignoring");
      return;
    }
    info!("Casting vote");
    self
      .votes
      .entry(vote.into())
      .and_modify(|e| match e.2.contains(voter) {
        false => {
          e.1 += 1;
          e.2.insert(voter.into());
        }
        true => {
          e.1 -= 1;
          e.2.remove(voter);
        }
      });
    self.set_highest_vote();
  }

  fn set_highest_vote(&mut self) {
    self.most_votes = self.votes.values().map(|e| e.1).max().unwrap_or(0);
  }
}
