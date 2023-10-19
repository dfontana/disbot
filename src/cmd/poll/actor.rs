use std::collections::HashMap;

use async_trait::async_trait;
use serenity::{
  model::prelude::{interaction::message_component::MessageComponentInteraction, ChannelId},
  prelude::Context,
};
use tokio::sync::{mpsc::Receiver, oneshot};
use tracing::error;
use uuid::Uuid;

use crate::actor::{Actor, ActorHandle};

use super::{messages, pollstate::PollState};

pub enum PollMessage {
  UpdateVote((Uuid, String, Context, MessageComponentInteraction)),
  CreatePoll((PollState, ChannelId)),
  ExpirePoll(Uuid),
  GetAdminState(oneshot::Sender<Vec<PollState>>),
}

pub struct PollActor {
  self_ref: ActorHandle<PollMessage>,
  receiver: Receiver<PollMessage>,
  states: HashMap<Uuid, PollState>,
}

impl PollActor {
  pub fn new(receiver: Receiver<PollMessage>, self_ref: ActorHandle<PollMessage>) -> Box<Self> {
    Box::new(Self {
      self_ref,
      receiver,
      states: HashMap::new(),
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
          .and_then(|_| {
            self.states.insert(ps.id, ps);
            Ok(())
          })
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
        let (resp, ctx) = match self.states.get(&id) {
          Some(p) => (messages::build_exp_message(p), p.ctx.clone()),
          None => {
            error!("Poll no longer exists for expiring: {}", id);
            return;
          }
        };
        self.states.remove(&id);
        let _ = ctx.channel.say(&ctx.http, resp).await;
      }
      PollMessage::GetAdminState(send) => {
        let msg = self.states.values().map(|v| v.clone()).collect();
        if let Err(_) = send.send(msg) {
          error!("Failed to send admin state, recv dropped");
        }
      }
      PollMessage::UpdateVote((id, voter, ctx, mtx)) => {
        if !self.states.contains_key(&id) {
          error!("Failed to cast vote for poll {}: Poll expired", id);
          return;
        }
        let p = self
          .states
          .entry(id.clone())
          .and_modify(|p| p.update_vote(&mtx.data.values, &voter))
          .or_insert_with(|| panic!("Entry went missing during update"));

        let new_body = messages::build_poll_message(p);

        if let Err(e) = mtx
          .message
          .clone()
          .edit(&ctx, |body| body.content(new_body))
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
