use crate::{
  actor::{Actor, ActorHandle},
  cmd::poll::PollMessage,
};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::America;
use derive_new::new;
use serenity::{
  http::Http,
  model::prelude::{ChannelId, Emoji},
};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::Receiver;
use tracing::{info, instrument};

#[derive(Clone)]
pub enum CheckInMessage {
  CheckIn(CheckInCtx),
  Sleep((Duration, CheckInCtx)),
  SetPoll(CheckInCtx),
}

#[derive(new, Clone)]
pub struct CheckInCtx {
  pub poll_time: NaiveTime,
  pub poll_dur: Duration,
  pub channel: ChannelId,
  pub http: Arc<Http>,
  pub emoji: Emoji,
}

pub struct CheckInActor {
  self_ref: ActorHandle<CheckInMessage>,
  receiver: Receiver<CheckInMessage>,
  poll_handle: ActorHandle<PollMessage>,
  configured: bool,
}

impl CheckInActor {
  pub fn new(
    self_ref: ActorHandle<CheckInMessage>,
    receiver: Receiver<CheckInMessage>,
    poll_handle: ActorHandle<PollMessage>,
  ) -> Self {
    CheckInActor {
      self_ref,
      receiver,
      poll_handle,
      configured: false,
    }
  }
}

#[async_trait]
impl Actor<CheckInMessage> for CheckInActor {
  #[instrument(name = "CheckIn", level = "INFO", skip(self, msg))]
  async fn handle_msg(&mut self, msg: CheckInMessage) {
    match msg {
      CheckInMessage::SetPoll(ctx) => {
        if self.configured {
          return;
        }
        let sleep_until = time_until(Utc::now(), ctx.poll_time);
        self
          .self_ref
          .send(CheckInMessage::Sleep((sleep_until, ctx)))
          .await;
        self.configured = true;
      }
      CheckInMessage::Sleep((sleep_until, ctx)) => {
        let hdl = self.self_ref.clone();
        info!(
          "Sleep scheduled until {}",
          Utc::now() + chrono::Duration::from_std(sleep_until).unwrap()
        );
        tokio::spawn(async move {
          tokio::time::sleep(sleep_until).await;
          hdl.send(CheckInMessage::CheckIn(ctx)).await
        });
      }
      CheckInMessage::CheckIn(ctx) => {
        let chan = ctx.channel;
        let nw_ctx = ctx.clone();
        self
          .poll_handle
          .send(PollMessage::CreatePoll((ctx.into(), chan)))
          .await;
        let sleep_until = time_until(Utc::now(), nw_ctx.poll_time);
        self
          .self_ref
          .send(CheckInMessage::Sleep((sleep_until, nw_ctx)))
          .await;
      }
    }
  }

  fn receiver(&mut self) -> &mut Receiver<CheckInMessage> {
    &mut self.receiver
  }
}

fn time_until(now_ref: DateTime<Utc>, time: NaiveTime) -> Duration {
  let now_local = now_ref.with_timezone(&America::New_York);
  let target_local = America::New_York
    .from_local_datetime(&NaiveDateTime::new(now_ref.naive_local().date(), time))
    .unwrap();

  let diff = now_local.signed_duration_since(target_local);
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

  use crate::cmd::check_in::actor::time_until;

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
