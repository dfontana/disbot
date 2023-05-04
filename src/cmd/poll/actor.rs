use serenity::{
  model::prelude::interaction::{
    application_command::ApplicationCommandInteraction,
    message_component::MessageComponentInteraction,
  },
  prelude::Context,
};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::{error, warn};
use uuid::Uuid;

use super::{cache::Cache, messages, pollstate::PollState};
pub enum PollMessage {
  UpdateVote((Uuid, String, Context, MessageComponentInteraction)),
  CreatePoll((PollState, ApplicationCommandInteraction)),
  ExpirePoll(Uuid),
}

#[derive(Clone)]
pub struct PollHandle {
  sender: Sender<PollMessage>,
}

async fn run_poller(mut actor: PollActor) {
  while let Some(msg) = actor.receiver.recv().await {
    actor.handle_msg(msg).await
  }
}

impl PollHandle {
  pub fn new() -> Self {
    let (sender, receiver) = mpsc::channel(8);
    let handle = Self { sender };
    let actor = PollActor::new(handle.clone(), receiver);
    tokio::spawn(run_poller(actor));
    handle
  }

  pub async fn send(&self, msg: PollMessage) {
    let _ = self.sender.send(msg).await;
  }
}

pub struct PollActor {
  self_ref: PollHandle,
  receiver: Receiver<PollMessage>,
  states: Cache<Uuid, PollState>,
}

impl PollActor {
  pub fn new(self_ref: PollHandle, receiver: Receiver<PollMessage>) -> Self {
    Self {
      self_ref,
      receiver,
      states: Cache::new(),
    }
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
              .invoke_mut(&id, |p| p.update_vote(&mtx.data.values, &voter))
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
