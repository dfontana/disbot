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
  cmd::{PollMessage, PollState, CheckInCtx}, ActorHandles,
};

#[derive(Clone)]
struct AppState {
  actors: ActorHandles,
}

impl FromRef<AppState> for ActorHandle<PollMessage> {
  fn from_ref(app_state: &AppState) -> ActorHandle<PollMessage> {
    app_state.actors.poll.clone()
  }
}

pub fn admin_routes(actors: ActorHandles) -> Router {
  Router::new()
    .route("/", get(polls))
    .route("/polls/:id", post(delete_poll))
    .with_state(AppState { actors })
}

#[derive(Template)]
#[template(path = "polls.html")]
struct Polls<'a> {
  title: &'a str,
  polls: Vec<PollState>,
  check_in: Option<CheckInCtx>,
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
    title: "Disbot Admin UI",
    polls: admin_info,
    check_in: None,
  })
}

async fn delete_poll(
  State(poll_handle): State<ActorHandle<PollMessage>>,
  Path(poll_id): Path<Uuid>,
) -> impl IntoResponse {
  poll_handle.send(PollMessage::ExpirePoll(poll_id)).await;
  Redirect::to("/ui")
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
