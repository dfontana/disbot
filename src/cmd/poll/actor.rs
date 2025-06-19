use async_trait::async_trait;
use serenity::{
  all::{ChannelId, ComponentInteraction, ComponentInteractionDataKind},
  builder::EditMessage,
  prelude::Context,
};
use std::{sync::Arc, time::SystemTime};
use tokio::sync::mpsc::Receiver;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
  actor::{Actor, ActorHandle},
  persistence::PersistentStore,
};

use super::{cache::Cache, messages, pollstate::PollState};

#[derive(Clone)]
pub enum PollMessage {
  UpdateVote((Uuid, String, Context, ComponentInteraction)),
  CreatePoll((PollState, ChannelId)),
  ExpirePoll(Uuid),
  RestorePolls(Arc<serenity::http::Http>),
}

pub struct PollActor {
  self_ref: ActorHandle<PollMessage>,
  receiver: Receiver<PollMessage>,
  states: Cache<Uuid, PollState>,
  persistence: Arc<PersistentStore>,
}

impl PollActor {
  pub fn new(
    receiver: Receiver<PollMessage>,
    self_ref: ActorHandle<PollMessage>,
    persistence: Arc<PersistentStore>,
  ) -> Box<Self> {
    Box::new(Self {
      self_ref,
      receiver,
      states: Cache::new(),
      persistence,
    })
  }
}

#[async_trait]
impl Actor<PollMessage> for PollActor {
  fn receiver(&mut self) -> &mut Receiver<PollMessage> {
    &mut self.receiver
  }

  async fn handle_msg(&mut self, msg: PollMessage) {
    match msg {
      PollMessage::CreatePoll((ps, itx)) => {
        let exp = ps.duration;
        let exp_key = ps.id;

        // Save to persistence first
        if let Err(e) = self.persistence.save_poll(&ps.id, &ps) {
          error!("Failed to persist poll {}: {}", ps.id, e);
        }

        if let Err(e) = messages::send_poll_message(&ps, &itx)
          .await
          .map_err(|e| format!("{}", e))
          .and_then(|_| self.states.insert(ps.id, ps))
        {
          error!("Failed to launch poll {}", e);
          return;
        }

        let hdl = self.self_ref.clone();
        tokio::spawn(async move {
          tokio::time::sleep(exp).await;
          hdl.send(PollMessage::ExpirePoll(exp_key)).await
        });
      }
      PollMessage::ExpirePoll(id) => {
        let (resp, ctx) = match self
          .states
          .invoke(&id, |p| (messages::build_exp_message(p), p.ctx.clone()))
        {
          Err(e) => {
            error!("Failed to inform channel poll has finished: {}", e);
            return;
          }
          Ok(v) => v,
        };

        let _ = ctx.channel.say(&ctx.http, resp).await;

        if let Err(e) = self.states.remove(&id) {
          warn!("Failed to reap poll on exp: {}", e);
        }

        if let Err(e) = self.persistence.remove_poll(&id) {
          error!(
            "Failed to remove expired poll {} from persistence: {}",
            id, e
          );
        }
      }
      PollMessage::UpdateVote((id, voter, ctx, mtx)) => {
        let votes = match mtx.data.kind {
          ComponentInteractionDataKind::StringSelect { ref values } => values,
          _ => {
            error!("Wrong interaction kind passed to UpdateVote");
            return;
          }
        };
        let new_body = match self
          .states
          .contains_key(&id)
          .and_then(|ext| match ext {
            true => Ok(()),
            false => Err(format!("Poll expired, dead interaction: {}", id)),
          })
          .and_then(|_| {
            self
              .states
              .invoke_mut(&id, |p| p.update_vote(votes, &voter))
          })
          .and_then(|_| {
            if let Err(e) = self
              .states
              .invoke(&id, |p| self.persistence.save_poll(&p.id, p))
            {
              warn!("Failed to persist vote update for poll {}: {}", id, e);
            }
            self.states.invoke(&id, messages::build_poll_message)
          }) {
          Ok(v) => v,
          Err(e) => {
            error!("Failed to cast vote for poll {}: {}", id, e);
            return;
          }
        };

        if let Err(e) = mtx
          .message
          .clone()
          .edit(&ctx, EditMessage::new().content(new_body))
          .await
        {
          error!("Failed to update the message body: {}", e);
          return;
        }

        if let Err(e) = mtx.defer(&ctx.http).await {
          error!("Failed to defer the interaction: {}", e);
        }
      }
      PollMessage::RestorePolls(http) => {
        match self.persistence.load_all_polls() {
          Ok(polls) => {
            for mut poll in polls {
              // Restore the Http client that was skipped during serialization
              poll.ctx.http = http.clone();

              // Check if poll has expired using checked duration calculation
              let elapsed = SystemTime::now()
                .duration_since(poll.created_at)
                .unwrap_or(poll.duration);
              if elapsed >= poll.duration {
                if let Err(e) = self.persistence.remove_poll(&poll.id) {
                  warn!("Failed to clean up expired poll from persistence: {}", e);
                }
                continue;
              }

              // Restore to in-memory cache
              let poll_id = poll.id;
              if let Err(e) = self.states.insert(poll.id, poll.clone()) {
                error!("Failed to restore poll {} to cache: {}", poll_id, e);
                if let Err(cleanup_err) = self.persistence.remove_poll(&poll_id) {
                  error!(
                    "Failed to clean up unrestorable poll {} from persistence: {}",
                    poll_id, cleanup_err
                  );
                }
                continue;
              }

              // Set up expiry timer for remaining duration using checked arithmetic
              let remaining_duration = poll.duration - elapsed;
              let exp_key = poll.id;
              let hdl = self.self_ref.clone();
              tokio::spawn(async move {
                tokio::time::sleep(remaining_duration).await;
                hdl.send(PollMessage::ExpirePoll(exp_key)).await
              });

              info!(
                "Restored poll {} with {} remaining",
                poll.id,
                humantime::format_duration(remaining_duration)
              );
            }
          }
          Err(e) => {
            error!("Failed to restore polls from persistence: {}", e);
          }
        }
      }
    }
  }
}
