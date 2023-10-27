use crate::{
  actor::{Actor, ActorHandle},
  cmd::poll::PollMessage,
};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::America;
use derive_new::new;
use humantime::parse_duration;
use serenity::{
  http::Http,
  model::prelude::{ChannelId, Emoji},
};
use std::{sync::Arc, time::Duration};
use tokio::{
  sync::{mpsc::Receiver, oneshot},
  task::JoinHandle,
};
use tracing::{error, info, instrument};

pub enum CheckInMessage {
  CheckIn(CheckInCtx),
  Sleep((Duration, CheckInCtx)),
  SetPoll(CheckInCtx),
  UpdatePoll((String, String, oneshot::Sender<Option<String>>)),
  GetAdminState(oneshot::Sender<Option<CheckInCtx>>),
  Cancel(oneshot::Sender<Option<String>>),
}

#[derive(new, Clone)]
pub struct CheckInCtx {
  pub poll_time: NaiveTime,
  pub poll_dur: Duration,
  pub channel: ChannelId,
  pub http: Arc<Http>,
  pub emoji: Emoji,
}

impl CheckInCtx {
  pub fn update(&self, time_s: String, dur_s: String) -> Result<Self, String> {
    CheckInCtx::parse(
      time_s,
      dur_s,
      self.channel.clone(),
      self.http.clone(),
      self.emoji.clone(),
    )
  }

  pub fn parse(
    time_s: String,
    dur_s: String,
    channel: ChannelId,
    http: Arc<Http>,
    emoji: Emoji,
  ) -> Result<Self, String> {
    Ok(CheckInCtx {
      poll_time: time_s
        .parse::<NaiveTime>()
        .map_err(|err| format!("Invalid time given {:?}. Cause: {}", time_s, err))?,
      poll_dur: parse_duration(&dur_s)
        .map_err(|err| format!("Invalid duration given {:?}. Cause: {}", dur_s, err))?,
      channel,
      http,
      emoji,
    })
  }
}

pub struct CheckInActor {
  self_ref: ActorHandle<CheckInMessage>,
  receiver: Receiver<CheckInMessage>,
  poll_handle: ActorHandle<PollMessage>,
  task: Option<(JoinHandle<()>, CheckInCtx)>,
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
      task: None,
    }
  }
}

#[async_trait]
impl Actor<CheckInMessage> for CheckInActor {
  #[instrument(name = "CheckIn", level = "INFO", skip(self, msg))]
  async fn handle_msg(&mut self, msg: CheckInMessage) {
    match msg {
      CheckInMessage::UpdatePoll((time_s, dur_s, send)) => {
        let maybe_new_ctx = self
          .task
          .as_ref()
          .ok_or("No registered check-in".into())
          .and_then(|(_, ctx)| ctx.update(time_s, dur_s));
        match maybe_new_ctx {
          Ok(ctx) => {
            self.self_ref.send(CheckInMessage::SetPoll(ctx)).await;
            let _ = send.send(None);
          }
          Err(err) => {
            let _ = send.send(Some(err));
          }
        }
      }
      CheckInMessage::Cancel(send) => {
        if let Some((t, _)) = self.task.as_ref() {
          info!("Cancelling existing task");
          t.abort();
          self.task = None;
          let _ = send.send(None);
        } else {
          let _ = send.send(Some("No registered check-in".into()));
        }
      }
      CheckInMessage::SetPoll(ctx) => {
        if let Some((t, _)) = self.task.as_ref() {
          info!("Cancelling existing task");
          t.abort();
          self.task = None;
        }
        let sleep_until = time_until(Utc::now(), ctx.poll_time);
        self
          .self_ref
          .send(CheckInMessage::Sleep((sleep_until, ctx)))
          .await;
      }
      CheckInMessage::Sleep((sleep_until, ctx)) => {
        let hdl = self.self_ref.clone();
        info!(
          "Sleep scheduled until {}",
          Utc::now() + chrono::Duration::from_std(sleep_until).unwrap()
        );
        let ctx_cpy = ctx.clone();
        self.task = Some((
          tokio::spawn(async move {
            tokio::time::sleep(sleep_until).await;
            hdl.send(CheckInMessage::CheckIn(ctx)).await
          }),
          ctx_cpy,
        ));
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
      CheckInMessage::GetAdminState(send) => {
        if let Err(_) = send.send(self.task.as_ref().map(|x| x.1.clone())) {
          error!("Failed to send admin state, recv dropped");
        }
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
