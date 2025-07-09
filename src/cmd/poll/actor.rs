use super::{cache::Cache, messages, pollstate::PollState};
use crate::{
  actor::{Actor, ActorHandle},
  cmd::{poll::NAME, CallContext},
  emoji::EmojiLookup,
  persistence::PersistentStore,
  shutdown::ShutdownHook,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serenity::{
  all::{ComponentInteraction, ComponentInteractionDataKind, Emoji, GuildId},
  builder::EditMessage,
  prelude::Context,
};
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

#[derive(Clone)]
pub enum PollMessage {
  UpdateVote(Box<(Uuid, String, Context, ComponentInteraction)>),
  CreatePoll(Box<(PollState, CallContext)>),
  ExpirePoll(Uuid, CallContext),
  RestorePolls(CallContext),
}

pub struct PollActor {
  self_ref: ActorHandle<PollMessage>,
  receiver: Receiver<PollMessage>,
  states: Cache<Uuid, PollState>,
  persistence: Arc<PersistentStore>,
  emoji: EmojiLookup,
}

impl PollActor {
  pub fn new(
    receiver: Receiver<PollMessage>,
    self_ref: ActorHandle<PollMessage>,
    persistence: Arc<PersistentStore>,
    emoji: EmojiLookup,
  ) -> Box<Self> {
    Box::new(Self {
      self_ref,
      receiver,
      states: Cache::new(),
      persistence,
      emoji,
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
        let (ps, ctx) = *boxed_data;
        let exp = ps.duration;
        let exp_key = *ps.id;

        // Save to persistence first
        if let Err(e) = self.persistence.polls().save(&ps.id, &ps) {
          error!("Failed to persist poll {}: {}", *ps.id, e);
        }

        let Some(emoji) = get_emoji(&self.emoji, Some(*ps.guild), &ctx).await else {
          return;
        };
        if let Err(e) = messages::send_poll_message(&ps, &ctx, &emoji)
          .await
          .map_err(|e| anyhow!("{}", e))
          .and_then(|_| self.states.insert(*ps.id, ps))
        {
          error!("Failed to launch poll {}", e);
          return;
        }

        let hdl = self.self_ref.clone();
        tokio::spawn(async move {
          tokio::time::sleep(exp).await;
          hdl.send(PollMessage::ExpirePoll(exp_key, ctx)).await
        });
      }
      PollMessage::ExpirePoll(id, ctx) => {
        let ps = match self.states.invoke(&id, |p| p.clone()) {
          Err(e) => {
            error!("Failed to inform channel poll has finished: {}", e);
            return;
          }
          Ok(v) => v,
        };
        let Some(emoji) = get_emoji(&self.emoji, Some(*ps.guild), &ctx).await else {
          return;
        };
        let resp = messages::build_exp_message(&ps, &emoji);
        let _ = ps.channel.say(&ctx.http, resp).await;

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
        let call_ctx = CallContext {
          http: ctx.http.clone(),
        };
        let Some(emoji) = get_emoji(&self.emoji, mtx.guild_id, &call_ctx).await else {
          return;
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
            self
              .states
              .invoke(&id, |ps| messages::build_poll_message(ps, &emoji))
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
      PollMessage::RestorePolls(ctx) => {
        if let Err(e) = self.persistence.polls().cleanup_expired() {
          warn!("Failed to clean up expired poll from persistence: {}", e);
        }
        match self.persistence.polls().load_all() {
          Ok(polls) => {
            for (_, poll) in polls {
              // Restore to in-memory cache
              let poll_id = *poll.id;
              if let Err(e) = self.states.insert(*poll.id, poll.clone()) {
                panic!("Failed to restore poll {} to cache: {}", poll_id, e);
              }

              // Set up expiry timer for remaining duration using checked arithmetic
              let remaining_duration = poll.duration - poll.elapsed();
              let exp_key = *poll.id;
              let hdl = self.self_ref.clone();
              let nw_ctx = ctx.clone();
              tokio::spawn(async move {
                tokio::time::sleep(remaining_duration).await;
                hdl.send(PollMessage::ExpirePoll(exp_key, nw_ctx)).await
              });

              info!(
                "Restored poll {} with {} remaining",
                *poll.id,
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

async fn get_emoji(
  emoji: &EmojiLookup,
  maybe_guild: Option<GuildId>,
  ctx: &CallContext,
) -> Option<Emoji> {
  let emoji_res = maybe_guild
    .ok_or(anyhow!("Missing guild_id from interaction"))
    .map(|gid| emoji.get(&ctx.http, gid));
  let emoji_res = match emoji_res {
    Ok(me) => me.await,
    Err(e) => {
      error!("Failed to get emoji {}", e);
      return None;
    }
  };
  match emoji_res {
    Ok(me) => Some(me),
    Err(e) => {
      error!("Failed to get emoji {}", e);
      None
    }
  }
}

#[async_trait]
impl ShutdownHook for PollActor {
  #[instrument(name=NAME, level="INFO", skip(self))]
  async fn shutdown(&self) -> Result<()> {
    info!("Shutting down");
    self.states.iter(|id, poll| {
      info!("Saving poll {}", id);
      if let Err(e) = self.persistence.polls().save(id, poll) {
        error!("Failed to save poll {} during shutdown: {}", id, e)
      }
    })?;
    info!("Shutdown complete");
    Ok(())
  }
}
