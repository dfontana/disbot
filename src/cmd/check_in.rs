use crate::{config::Config, emoji::EmojiLookup};
use chrono::{DateTime, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::{America, Tz};
use derive_new::new;
use std::time::Duration;

use super::poll::PollHandle;

#[derive(new)]
pub struct CheckIn {
  config: Config,
  emoji: EmojiLookup,
  poll_handle: PollHandle,
}

impl CheckIn {
  pub fn boot(self) {
    // TODO:
    // Spawn a sleep task (like poll expire does) to trigger the next poll.
    // This task should spawn the next task to sleep again, infinitum.
    // You need to avoid pushing the call stack on this, bear in mind
    let _sleep_until = time_until(Utc::now(), self.config.check_in.time);
    self.config.check_in.duration;
  }
}

fn time_until(now_ref: DateTime<Utc>, time: NaiveTime) -> Duration {
  let now_local = now_ref.with_timezone(&America::New_York);
  let target_local = America::New_York
    .from_local_datetime(&NaiveDateTime::new(now_ref.naive_local().date(), time))
    .unwrap();

  let diff = now_local.signed_duration_since(target_local);
  println!("{:?} {:?} {:?}", now_local, target_local, diff);
  match diff.cmp(&chrono::Duration::zero()) {
    std::cmp::Ordering::Less => (target_local - now_local).to_std().unwrap(),
    std::cmp::Ordering::Equal => std::time::Duration::default(),
    std::cmp::Ordering::Greater => {
      // Time has passed, schedule for tomrorow
      (target_local + chrono::Duration::days(1) - now_local)
        .to_std()
        .unwrap()
    }
  }
}

#[cfg(test)]
mod test {
  use std::{str::FromStr, time::Duration};

  use chrono::{DateTime, NaiveTime, Utc};

  use crate::cmd::check_in::time_until;

  #[test]
  fn time_in_past() {
    // This is actually 15 - 4 => 11:00
    let now: DateTime<Utc> = DateTime::from_str("2023-05-05T15:00:00Z").unwrap();
    let one_hour_ago = NaiveTime::from_str("10:00:00").unwrap();
    assert_eq!(Duration::from_secs(82800), time_until(now, one_hour_ago))
  }

  #[test]
  fn time_in_future() {
    // This is actually 15 - 4 => 11:00
    let now: DateTime<Utc> = DateTime::from_str("2023-05-05T15:00:00Z").unwrap();
    let one_hour_later = NaiveTime::from_str("12:00:00").unwrap();
    assert_eq!(Duration::from_secs(3600), time_until(now, one_hour_later))
  }

  #[test]
  fn time_is_now() {
    // This is actually 15 - 4 => 11:00
    let now: DateTime<Utc> = DateTime::from_str("2023-05-05T15:00:00Z").unwrap();
    let same_as_now = NaiveTime::from_str("11:00:00").unwrap();
    assert_eq!(Duration::from_secs(0), time_until(now, same_as_now))
  }
}
