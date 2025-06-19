use crate::{
  actor::{Actor, ActorHandle},
  cmd::poll::PollMessage,
  persistence::PersistentStore,
};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::America;
use derive_new::new;
use serde::{Deserialize, Serialize};
use serenity::{
  all::Role,
  http::Http,
  model::prelude::{ChannelId, Emoji},
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::mpsc::Receiver;
use tracing::{error, info, instrument};

#[derive(Clone)]
pub enum CheckInMessage {
  CheckIn(CheckInCtx),
  Sleep((Duration, CheckInCtx)),
  SetPoll(CheckInCtx),
  RestoreConfig(u64, Arc<serenity::http::Http>), // guild_id, http
}

#[derive(new, Clone, Serialize, Deserialize)]
pub struct CheckInCtx {
  pub poll_time: NaiveTime,
  pub poll_dur: Duration,
  pub at_group: Option<Role>,
  pub channel: ChannelId,
  #[serde(skip, default = "default_http")]
  pub http: Arc<Http>,
  pub emoji: Emoji,
  pub guild_id: u64,
}

fn default_http() -> Arc<Http> {
  // Create Http client with placeholder token that will be replaced during restoration
  // Using a non-empty token to avoid potential auth failures during serialization
  Arc::new(serenity::http::Http::new(
    "PLACEHOLDER_TOKEN_FOR_SERIALIZATION",
  ))
}

pub struct CheckInActor {
  self_ref: ActorHandle<CheckInMessage>,
  receiver: Receiver<CheckInMessage>,
  poll_handle: ActorHandle<PollMessage>,
  configured_guilds: HashMap<u64, bool>,
  persistence: Arc<PersistentStore>,
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
      configured_guilds: HashMap::new(),
      persistence,
    }
  }
}

#[async_trait]
impl Actor<CheckInMessage> for CheckInActor {
  #[instrument(name = "CheckIn", level = "INFO", skip(self, msg))]
  async fn handle_msg(&mut self, msg: CheckInMessage) {
    match msg {
      CheckInMessage::SetPoll(ctx) => {
        // Check if this guild is already configured
        if *self.configured_guilds.get(&ctx.guild_id).unwrap_or(&false) {
          info!(
            "Guild {} already has check-in configured, ignoring",
            ctx.guild_id
          );
          return;
        }

        // Persist the check-in configuration using the guild_id from context
        if let Err(e) = self.persistence.save_checkin_config(ctx.guild_id, &ctx) {
          error!(
            "Failed to persist check-in configuration for guild {}: {}",
            ctx.guild_id, e
          );
        }

        let sleep_until = time_until(Utc::now(), ctx.poll_time);
        self
          .self_ref
          .send(CheckInMessage::Sleep((sleep_until, ctx.clone())))
          .await;
        self.configured_guilds.insert(ctx.guild_id, true);
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
      CheckInMessage::RestoreConfig(guild_id, http) => {
        match self.persistence.load_checkin_config(guild_id) {
          Ok(Some(mut config)) => {
            // Restore the Http client that was skipped during serialization
            config.http = http;

            // Set up the restored configuration
            let sleep_until = time_until(Utc::now(), config.poll_time);
            self
              .self_ref
              .send(CheckInMessage::Sleep((sleep_until, config.clone())))
              .await;
            self.configured_guilds.insert(guild_id, true);

            info!("Restored check-in configuration for guild {}", guild_id);
          }
          Ok(None) => {
            info!("No check-in configuration found for guild {}", guild_id);
          }
          Err(e) => {
            error!(
              "Failed to restore check-in configuration for guild {}: {}",
              guild_id, e
            );
          }
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
