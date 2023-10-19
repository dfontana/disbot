use async_trait::async_trait;
use tokio::sync::mpsc::{self, Receiver, Sender};

// https://ryhl.io/blog/actors-with-tokio/
#[async_trait]
pub trait Actor<T: Send + Sync> {
  async fn handle_msg(&mut self, msg: T);
  fn receiver(&mut self) -> &mut Receiver<T>;
}

pub struct ActorHandle<T> {
  sender: Sender<T>,
}

// Cannot derive clone, because https://github.com/rust-lang/rust/issues/26925
// Manual impl avoids need to bound T: Clone, which makes no sense
impl<T> Clone for ActorHandle<T> {
  fn clone(&self) -> Self {
    ActorHandle {
      sender: self.sender.clone(),
    }
  }
}

impl<T: Send + Sync + 'static> ActorHandle<T> {
  pub fn spawn(
    mk_actor: impl Fn(Receiver<T>, ActorHandle<T>) -> Box<dyn Actor<T> + Send + Sync>,
  ) -> Self {
    let (sender, receiver) = mpsc::channel(8);
    let handle = Self { sender };
    let actor = mk_actor(receiver, handle.clone());
    tokio::spawn(run_actor(actor));
    handle
  }

  pub async fn send(&self, msg: T) {
    let _ = self.sender.send(msg).await;
  }
}

async fn run_actor<T: Send + Sync>(mut actor: Box<dyn Actor<T> + Send + Sync>) {
  while let Some(msg) = actor.receiver().recv().await {
    actor.handle_msg(msg).await
  }
}
