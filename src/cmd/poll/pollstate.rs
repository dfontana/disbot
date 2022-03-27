use humantime::parse_duration;
use serenity::framework::standard::Args;
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
  pub fn from_args(mut args: Args) -> Result<PollState, String> {
    let duration = parse_duration(&args.single_quoted::<String>().unwrap())
      .map_err(|_| "Invalid duration given")?;
    let topic = args.single_quoted::<String>().unwrap();
    let items: Vec<String> = args
      .trimmed()
      .quoted()
      .iter::<String>()
      .map(|arg| arg.unwrap().trim_matches('"').to_owned())
      .collect();
    if items.len() > 10 {
      return Err("Too many arguments given".to_string());
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
