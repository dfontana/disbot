use async_trait::async_trait;
use serenity::{
  all::{ChannelId, ComponentInteraction, ComponentInteractionDataKind},
  builder::EditMessage,
  prelude::Context,
};
use tokio::sync::mpsc::Receiver;
use tracing::{error, warn};
use uuid::Uuid;

use crate::actor::{Actor, ActorHandle};

use super::{cache::Cache, messages, pollstate::PollState};

#[derive(Clone)]
pub enum PollMessage {
  UpdateVote((Uuid, String, Context, ComponentInteraction)),
  CreatePoll((PollState, ChannelId)),
  ExpirePoll(Uuid),
}

pub struct PollActor {
  self_ref: ActorHandle<PollMessage>,
  receiver: Receiver<PollMessage>,
  states: Cache<Uuid, PollState>,
}

impl PollActor {
  pub fn new(receiver: Receiver<PollMessage>, self_ref: ActorHandle<PollMessage>) -> Box<Self> {
    Box::new(Self {
      self_ref,
      receiver,
      states: Cache::new(),
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
          .and_then(|_| self.states.invoke(&id, messages::build_poll_message))
        {
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
    }
  }
}
