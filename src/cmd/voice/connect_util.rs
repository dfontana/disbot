use once_cell::sync::Lazy;
use serenity::{
  async_trait,
  http::Http,
  model::{
    guild::Emoji,
    id::{ChannelId, GuildId},
  },
  prelude::{Mutex, RwLock},
  utils::MessageBuilder,
  FutureExt,
};
use songbird::{Call, Event, EventContext, EventHandler, Songbird};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::{info, instrument};

static HANDLER_ADDED: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(false));
static CHAN: Lazy<(Sender<bool>, Mutex<Receiver<bool>>)> = Lazy::new(|| {
  let (sd, rc) = mpsc::channel(32);
  (sd, Mutex::new(rc))
});
static IN_PROG_COUNT: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
const TIMEOUT_SECS: u64 = 600;

#[derive(Builder, Clone)]
pub struct ChannelDisconnect {
  manager: Arc<Songbird>,
  http: Arc<Http>,
  guild: GuildId,
  channel: ChannelId,
  emoji: Emoji,
}

impl ChannelDisconnect {
  pub fn get_chan() -> Sender<bool> {
    CHAN.0.clone()
  }

  pub async fn maybe_register_handler(&self, handler_lock: &Arc<Mutex<Call>>) {
    if !HANDLER_ADDED.read().map(|g| *g).await {
      let _fut = HANDLER_ADDED.write().map(|mut g| *g = true).await;
      let mut handler = handler_lock.lock().await;
      handler.add_global_event(
        Event::Periodic(Duration::from_secs(TIMEOUT_SECS), None),
        self.clone(),
      );
      tokio::spawn(async move {
        loop {
          match CHAN.1.lock().await.recv().await {
            Some(true) => *(IN_PROG_COUNT.lock().await) += 1,
            Some(false) => *(IN_PROG_COUNT.lock().await) -= 1,
            None => (),
          }
        }
      });
    }
  }

  pub async fn stop(&self) {
    let _dis = self.disconnect(true).await;
  }

  async fn is_queuing(&self) -> bool {
    let count = *IN_PROG_COUNT.lock().await;
    info!("Checking for in prog: {}", count);
    count > 0
  }

  async fn disconnect(&self, force: bool) {
    let should_close = match self.manager.get(self.guild) {
      None => {
        info!("Not in a voice channel");
        let _ = self.channel.say(&self.http, "Not in a voice channel").await;
        false
      }
      Some(handler_lock) => {
        let handler = handler_lock.lock().await;
        if force {
          info!("Stopping queue");
          handler.queue().stop();
          true
        } else {
          info!("Checking queue prescense");
          handler.queue().is_empty() && !self.is_queuing().await
        }
      }
    };
    if should_close {
      info!("Disconnecting client from voice");
      let _dc = self.manager.leave(self.guild).await;
      let _rep = self
        .channel
        .say(
          &self.http,
          MessageBuilder::new()
            .mention(&self.emoji)
            .push(" Cya later NERD ")
            .mention(&self.emoji)
            .build(),
        )
        .await;
      info!("Disconnected");
    }
  }
}

#[async_trait]
impl EventHandler for ChannelDisconnect {
  #[instrument(name = "VoiceTimeoutListener", level = "INFO", skip(self, _ctx))]
  async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
    info!("Checking for inactivity...");
    let _dis = self.disconnect(false).await;
    None
  }
}
