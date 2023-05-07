use derive_new::new;
use serenity::{
  async_trait,
  http::Http,
  model::{guild::Emoji, id::ChannelId},
  prelude::Mutex,
  utils::MessageBuilder,
};
use songbird::{Call, Event, EventContext, EventHandler};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::Receiver;
use tracing::{info, instrument};

use crate::actor::{Actor, ActorHandle};

#[derive(Clone)]
pub enum DisconnectMessage {
  Enqueue,
  Dequeue,
  Details(DisconnectDetails),
  Disconnect(bool), // Forced = true
}

#[derive(new, Clone)]
pub struct DisconnectDetails {
  voice: Arc<Mutex<Call>>,
  http: Arc<Http>,
  emoji: Emoji,
}

pub struct DisconnectActor {
  receiver: Receiver<DisconnectMessage>,
  in_progress_count: usize,
  disconnect_details: Mutex<Option<DisconnectDetails>>,
}

#[async_trait]
impl Actor<DisconnectMessage> for DisconnectActor {
  fn receiver(&mut self) -> &mut Receiver<DisconnectMessage> {
    &mut self.receiver
  }

  async fn handle_msg(&mut self, msg: DisconnectMessage) {
    match msg {
      DisconnectMessage::Enqueue => self.in_progress_count += 1,
      DisconnectMessage::Dequeue => self.in_progress_count -= 1,
      DisconnectMessage::Details(det) => {
        let mut det_lock = self.disconnect_details.lock().await;
        *det_lock = Some(det);
      }
      DisconnectMessage::Disconnect(forced) => self.disconnect(forced).await,
    }
  }
}

impl DisconnectActor {
  pub fn new(receiver: Receiver<DisconnectMessage>) -> Self {
    Self {
      receiver,
      in_progress_count: 0,
      disconnect_details: Mutex::new(None),
    }
  }

  async fn disconnect(&mut self, force: bool) {
    let det_lock = self.disconnect_details.lock().await;
    let Some(details) = det_lock.as_ref() else {
      // Nothing to disconnect from silly
      info!("No disconnect details present, nothing to disconnect");
      return;
    };
    let mut handler = details.voice.lock().await;

    let Some(channel) = handler.current_channel() else {
      info!("Not in a channel, nothing to disconnect");
      return;
    };

    if force {
      info!("Force stopping");
      handler.queue().stop();
    } else if !handler.queue().is_empty() || self.in_progress_count != 0 {
      info!(
        "Queue not empty or queuing ({}), will not disconnect",
        self.in_progress_count
      );
      return;
    }

    info!("Disconnecting client from voice");
    let s_channel = ChannelId::from(channel.0);
    let _dc = handler.leave().await;
    let _rep = s_channel
      .say(
        &details.http,
        MessageBuilder::new()
          .mention(&details.emoji)
          .push(" Cya later NERD ")
          .mention(&details.emoji)
          .build(),
      )
      .await;
    info!("Disconnected");

    // Don't reset the details since the call is still valid, and may reconnect
    // we'll let it tell us when to. On the contrary, though, we should reset queuing
    // progress
    self.in_progress_count = 0;
  }
}

pub struct DisconnectEventHandler {
  handle: ActorHandle<DisconnectMessage>,
}

impl DisconnectEventHandler {
  pub async fn register(
    timeout: u64,
    handle: ActorHandle<DisconnectMessage>,
    call: &Arc<Mutex<Call>>,
  ) {
    let mut call_lock = call.lock().await;
    call_lock.add_global_event(
      Event::Periodic(Duration::from_secs(timeout), None),
      Self { handle },
    );
  }
}

#[async_trait]
impl EventHandler for DisconnectEventHandler {
  #[instrument(name = "VoiceTimeoutListener", level = "INFO", skip(self, _ctx))]
  async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
    info!("Checking for inactivity...");
    let _ = self.handle.send(DisconnectMessage::Disconnect(false)).await;
    None
  }
}
