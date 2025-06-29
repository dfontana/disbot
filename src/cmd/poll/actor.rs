use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serenity::{
  all::{ChannelId, ComponentInteraction, ComponentInteractionDataKind},
  builder::EditMessage,
  prelude::Context,
};
use std::{sync::Arc, time::SystemTime};
use tokio::sync::mpsc::Receiver;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::{
  actor::{Actor, ActorHandle},
  cmd::poll::NAME,
  persistence::PersistentStore,
  shutdown::ShutdownHook,
};

use super::{cache::Cache, messages, pollstate::PollState};

#[derive(Clone)]
pub enum PollMessage {
  UpdateVote(Box<(Uuid, String, Context, ComponentInteraction)>),
  CreatePoll(Box<(PollState, ChannelId)>),
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

  #[instrument(name=NAME, level="INFO", skip(self, msg))]
  async fn handle_msg(&mut self, msg: PollMessage) {
    match msg {
      PollMessage::CreatePoll(boxed_data) => {
        let (ps, itx) = *boxed_data;
        let exp = ps.duration;
        let exp_key = ps.id;

        // Save to persistence first
        // TODO: This is the only time a poll is saved to disk, so any votes captured
        //    between the time the poll was made and expired would be lost on restore.
        //    (Verify this in dev server and then consider a fix). You likely want
        //    to support a shutdown hook so things like polls and chat sessions
        //    (and other handlers) can persist their state
        if let Err(e) = self.persistence.polls().save(&ps.id, &ps) {
          error!("Failed to persist poll {}: {}", ps.id, e);
        }

        if let Err(e) = messages::send_poll_message(&ps, &itx)
          .await
          .map_err(|e| anyhow!("{}", e))
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

        if let Err(e) = self.persistence.polls().remove(&id) {
          error!(
            "Failed to remove expired poll {} from persistence: {}",
            id, e
          );
        }
      }
      PollMessage::UpdateVote(boxed_data) => {
        let (id, voter, ctx, mtx) = *boxed_data;
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
            false => Err(anyhow!("Poll expired, dead interaction: {}", id)),
          })
          .and_then(|_| {
            self
              .states
              .invoke_mut(&id, |p| p.update_vote(votes, &voter))
          })
          .and_then(|_| {
            if let Err(e) = self
              .states
              .invoke(&id, |p| self.persistence.polls().save(&p.id, p))
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
        match self.persistence.polls().load_all() {
          Ok(polls) => {
            for (_, mut poll) in polls {
              // Restore the Http client that was skipped during serialization
              poll.ctx.http = http.clone();

              // TODO: Should utilize the expiration feature of persistence to flush old data out on restore
              // Check if poll has expired using checked duration calculation
              let elapsed = SystemTime::now()
                .duration_since(poll.created_at)
                .unwrap_or(poll.duration);
              if elapsed >= poll.duration {
                if let Err(e) = self.persistence.polls().remove(&poll.id) {
                  warn!("Failed to clean up expired poll from persistence: {}", e);
                }
                continue;
              }

              // Restore to in-memory cache
              let poll_id = poll.id;
              if let Err(e) = self.states.insert(poll.id, poll.clone()) {
                error!("Failed to restore poll {} to cache: {}", poll_id, e);
                if let Err(cleanup_err) = self.persistence.polls().remove(&poll_id) {
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

#[async_trait]
impl ShutdownHook for PollActor {
  #[instrument(name=NAME, level="INFO", skip(self))]
  async fn shutdown(&self) -> Result<()> {
    info!("Shutting down");
    self.states.iter(|id, poll| {
      if let Err(e) = self.persistence.polls().save(id, poll) {
        error!("Failed to save poll {} during shutdown: {}", id, e)
      }
    })?;
    info!("Shutdown complete");
    Ok(())
  }
}
