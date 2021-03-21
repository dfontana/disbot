use crate::debug::Debug;
use serenity::framework::standard::Args;
use std::collections::HashMap;

pub struct PollState {
  pub topic: String,
  pub longest_option: usize,
  pub most_votes: usize,
  pub votes: HashMap<usize, (String, usize)>,
}

impl PollState {
  pub fn from_args(mut args: Args) -> Result<PollState, String> {
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

  pub fn cast_vote(&mut self, vote: usize) {
    if !self.votes.contains_key(&vote) {
      Debug::inst("poller").log("Vote not present in poll, ignoring");
      return;
    }
    self.votes.entry(vote).and_modify(|e| e.1 += 1);
    self.set_highest_vote();
  }

  pub fn revoke_vote(&mut self, vote: usize) {
    if !self.votes.contains_key(&vote) {
      Debug::inst("poller").log("Vote not present in poll, ignoring");
      return;
    }
    self.votes.entry(vote).and_modify(|e| e.1 -= 1);
    self.set_highest_vote();
  }

  fn set_highest_vote(&mut self) {
    self.most_votes = self.votes.values().map(|e| e.1).max().unwrap_or(0);
  }
}
