use crate::{
  cmd::{check_in::NAME, poll::PollMessage, CallContext},
  persistence::PersistentStore,
  types::{Chan, Guil, NaiveT, Rol},
};
use async_trait::async_trait;
use bincode::{Decode, Encode};
use chrono::{DateTime, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::America;
use derive_new::new;
use kitchen_sink::{
  actor::{Actor, ActorHandle},
  shutdown::ShutdownHook,
};
use serenity::all::GuildId;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::mpsc::Receiver, task::JoinHandle};
use tracing::{error, info, instrument};

#[derive(Clone)]
pub enum CheckInMessage {
  CheckIn(CheckInCtx, CallContext),
  Sleep((Duration, CheckInCtx, CallContext)),
  SetPoll(CheckInCtx, CallContext),
  RestoreConfig(CallContext),
}

#[derive(new, Clone, Encode, Decode)]
pub struct CheckInCtx {
  pub poll_time: NaiveT,
  pub poll_dur: Duration,
  pub at_group: Option<Rol>,
  pub channel: Chan,
  pub guild: Guil,
}

pub struct CheckInActor {
  self_ref: ActorHandle<CheckInMessage>,
  receiver: Receiver<CheckInMessage>,
  poll_handle: ActorHandle<PollMessage>,
  persistence: Arc<PersistentStore>,
  active_tasks: HashMap<GuildId, JoinHandle<()>>,
}

impl CheckInActor {
  pub fn new(
    self_ref: ActorHandle<CheckInMessage>,
    receiver: Receiver<CheckInMessage>,
    poll_handle: ActorHandle<PollMessage>,
    persistence: Arc<PersistentStore>,
  ) -> Self {
    CheckInActor {
      self_ref,
      receiver,
      poll_handle,
      persistence,
      active_tasks: HashMap::new(),
    }
  }
}

impl ShutdownHook for CheckInActor {}

#[async_trait]
impl Actor<CheckInMessage> for CheckInActor {
  #[instrument(name = NAME, level = "INFO", skip(self, msg))]
  async fn handle_msg(&mut self, msg: CheckInMessage) {
    match msg {
      CheckInMessage::SetPoll(ctx, cctx) => {
        // Cancel any existing sleep task for this guild
        if let Some(existing_task) = self.active_tasks.remove(&ctx.guild) {
          existing_task.abort();
          info!("Cancelled existing check-in task for guild {}", *ctx.guild);
        }

        // Persist the check-in configuration using the guild_id from context
        if let Err(e) = self.persistence.check_ins().save(&ctx.guild, &ctx) {
          error!(
            "Failed to persist check-in configuration for guild {}: {}",
            *ctx.guild, e
          );
        }

        let sleep_until = time_until(Utc::now(), *ctx.poll_time);
        self
          .self_ref
          .send(CheckInMessage::Sleep((sleep_until, ctx.clone(), cctx)))
          .await;
      }
      CheckInMessage::Sleep((sleep_until, ctx, cctx)) => {
        let hdl = self.self_ref.clone();
        let guild_id = *ctx.guild;
        info!(
          "Sleep scheduled until {}",
          Utc::now() + chrono::Duration::from_std(sleep_until).unwrap()
        );

        let task = tokio::spawn(async move {
          tokio::time::sleep(sleep_until).await;
          hdl.send(CheckInMessage::CheckIn(ctx, cctx)).await
        });

        // Store the task handle for potential cancellation
        self.active_tasks.insert(guild_id, task);
      }
      CheckInMessage::CheckIn(ctx, cctx) => {
        // Remove the completed task from active tasks
        self.active_tasks.remove(&ctx.guild);

        let nw_ctx = ctx.clone();
        self
          .poll_handle
          .send(PollMessage::CreatePoll(Box::new((
            ctx.into(),
            cctx.clone(),
          ))))
          .await;
        let sleep_until = time_until(Utc::now(), *nw_ctx.poll_time);
        self
          .self_ref
          .send(CheckInMessage::Sleep((sleep_until, nw_ctx, cctx)))
          .await;
      }
      CheckInMessage::RestoreConfig(cctx) => {
        let state = match self.persistence.check_ins().load_all() {
          Ok(v) => v,
          Err(e) => {
            error!("Failed to load check-in state {}", e);
            return;
          }
        };
        for (guild_id, config) in state {
          // Restore the Http client that was skipped during serialization
          let sleep_until = time_until(Utc::now(), *config.poll_time);
          self
            .self_ref
            .send(CheckInMessage::Sleep((
              sleep_until,
              config.clone(),
              cctx.clone(),
            )))
            .await;
          info!("Restored check-in configuration for guild {}", guild_id);
        }
      }
    }
  }

  fn receiver(&mut self) -> &mut Receiver<CheckInMessage> {
    &mut self.receiver
  }
}

pub fn time_until(now_ref: DateTime<Utc>, time: NaiveTime) -> Duration {
  let now_local = now_ref.with_timezone(&America::New_York);
  let target_local = America::New_York
    .from_local_datetime(&NaiveDateTime::new(now_local.date_naive(), time))
    .unwrap();

  let diff = now_local.signed_duration_since(target_local);
  match diff.cmp(&chrono::Duration::zero()) {
    std::cmp::Ordering::Less => (target_local - now_local).to_std().unwrap(),
    std::cmp::Ordering::Equal => std::time::Duration::default(),
    std::cmp::Ordering::Greater => {
      // Time has passed, schedule for tomorrow
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
