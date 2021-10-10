use std::{sync::Arc, time::Duration};

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
use tracing::{info, instrument};

lazy_static! {
  static ref HANDLER_ADDED: RwLock<bool> = RwLock::new(false);
}

#[derive(Builder, Clone)]
pub struct ChannelDisconnect {
  manager: Arc<Songbird>,
  http: Arc<Http>,
  guild: GuildId,
  channel: ChannelId,
  emoji: Emoji,
}

impl ChannelDisconnect {
  pub async fn maybe_register_handler(&self, handler_lock: &Arc<Mutex<Call>>) {
    if !HANDLER_ADDED.read().map(|g| *g).await {
      let _fut = HANDLER_ADDED.write().map(|mut g| *g = true).await;
      let mut handler = handler_lock.lock().await;
      handler.add_global_event(
        Event::Periodic(Duration::from_secs(300), None),
        self.clone(),
      );
    }
  }

  pub async fn stop(&self) {
    let _dis = self.disconnect(true).await;
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
          handler.queue().is_empty()
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
