use std::error::Error;

use askama_axum::Template;
use axum::{
  extract::{FromRef, Path, State},
  response::{Html, IntoResponse, Redirect, Response},
  routing::{get, post},
  Router,
};
use serde::Serialize;
use tokio::sync::oneshot;
use tracing::error;
use uuid::Uuid;

use crate::{
  actor::ActorHandle,
  cmd::{PollMessage, PollState},
};

#[derive(Clone)]
struct AppState {
  poll_handle: ActorHandle<PollMessage>,
}

impl FromRef<AppState> for ActorHandle<PollMessage> {
  fn from_ref(app_state: &AppState) -> ActorHandle<PollMessage> {
    app_state.poll_handle.clone()
  }
}

pub fn admin_routes(poll_handle: ActorHandle<PollMessage>) -> Router {
  Router::new()
    .route("/", get(index))
    .route("/polls", get(polls))
    .route("/polls/:id", post(delete_poll))
    .with_state(AppState { poll_handle })
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
  title: &'a str,
  pages: Vec<&'a str>,
}

async fn index() -> impl IntoResponse {
  Index {
    title: "Disbot Admin UI",
    pages: vec!["polls"],
  }
}

#[derive(Template)]
#[template(path = "polls.html")]
struct Polls<'a> {
  title: &'a str,
  polls: Vec<PollState>,
}

#[derive(Serialize)]
struct PollInfo {
  id: String,
  duration: String,
  topic: String,
}

async fn polls(
  State(poll_handle): State<ActorHandle<PollMessage>>,
) -> Result<Polls<'static>, HtmlErr> {
  let (send, recv) = oneshot::channel();
  poll_handle.send(PollMessage::GetAdminState(send)).await;
  let admin_info = match recv.await {
    Err(_) => return Err(HtmlErr::Rendering("no data".into())),
    Ok(v) => v,
  };

  Ok(Polls {
    title: "Poll Controls",
    polls: admin_info,
  })
}

async fn delete_poll(
  State(poll_handle): State<ActorHandle<PollMessage>>,
  Path(poll_id): Path<Uuid>,
) -> impl IntoResponse {
  poll_handle.send(PollMessage::ExpirePoll(poll_id)).await;
  Redirect::to("/ui/polls")
}

enum HtmlErr {
  Rendering(Box<dyn Error>),
}

impl IntoResponse for HtmlErr {
  fn into_response(self) -> Response {
    match self {
      HtmlErr::Rendering(err) => {
        error!("{}", err);
        Html("failed").into_response()
      }
    }
  }
}
