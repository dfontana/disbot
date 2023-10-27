use std::error::Error;

use askama_axum::Template;
use axum::{
  extract::{FromRef, Path, State},
  response::{Html, IntoResponse, Redirect, Response},
  routing::{get, post},
  Form, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tracing::error;
use uuid::Uuid;

use crate::{
  actor::ActorHandle,
  cmd::{CheckInCtx, CheckInMessage, PollMessage, PollState},
  ActorHandles,
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

impl FromRef<AppState> for ActorHandle<CheckInMessage> {
  fn from_ref(app_state: &AppState) -> ActorHandle<CheckInMessage> {
    app_state.actors.chk.clone()
  }
}

pub fn admin_routes(actors: ActorHandles) -> Router {
  Router::new()
    .route("/", get(index))
    .route("/polls/:id", post(delete_poll))
    .route("/check-in/update", get(update_check_in))
    .route("/check-in/cancel", post(delete_check_in))
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

async fn index(
  State(poll_handle): State<ActorHandle<PollMessage>>,
  State(chk_handle): State<ActorHandle<CheckInMessage>>,
) -> Result<Polls<'static>, HtmlErr> {
  let (send, recv) = oneshot::channel();
  poll_handle.send(PollMessage::GetAdminState(send)).await;
  let admin_info = match recv.await {
    Err(_) => return Err(HtmlErr::Rendering("no data".into())),
    Ok(v) => v,
  };

  let (send_chk, recv_chk) = oneshot::channel();
  chk_handle
    .send(CheckInMessage::GetAdminState(send_chk))
    .await;
  let chk_admin_info = match recv_chk.await {
    Err(_) => return Err(HtmlErr::Rendering("no data".into())),
    Ok(v) => v,
  };
  Ok(Polls {
    title: "Disbot Admin UI",
    polls: admin_info,
    check_in: chk_admin_info,
  })
}

async fn delete_poll(
  State(poll_handle): State<ActorHandle<PollMessage>>,
  Path(poll_id): Path<Uuid>,
) -> impl IntoResponse {
  poll_handle.send(PollMessage::ExpirePoll(poll_id)).await;
  Redirect::to("/ui")
}

#[derive(Deserialize)]
struct UpdateCheckIn {
  datetime: String,
  duration: String,
}

async fn update_check_in(
  State(chk_handle): State<ActorHandle<CheckInMessage>>,
  Form(updates): Form<UpdateCheckIn>,
) -> impl IntoResponse {
  let (send, recv) = oneshot::channel();
  chk_handle
    .send(CheckInMessage::UpdatePoll((
      updates.datetime,
      updates.duration,
      send,
    )))
    .await;
  match recv.await {
    Ok(Some(err)) => return Err(HtmlErr::Internal(err)),
    Ok(None) => Ok(Redirect::to("/ui")),
    Err(err) => return Err(HtmlErr::Internal(err.to_string())),
  }
}

async fn delete_check_in(
  State(chk_handle): State<ActorHandle<CheckInMessage>>,
) -> impl IntoResponse {
  let (send, recv) = oneshot::channel();
  chk_handle.send(CheckInMessage::Cancel(send)).await;
  match recv.await {
    Ok(Some(err)) => return Err(HtmlErr::Internal(err)),
    Ok(None) => Ok(Redirect::to("/ui")),
    Err(err) => return Err(HtmlErr::Internal(err.to_string())),
  }
}

enum HtmlErr {
  Rendering(Box<dyn Error>),
  Internal(String),
}

impl IntoResponse for HtmlErr {
  fn into_response(self) -> Response {
    match self {
      HtmlErr::Rendering(err) => {
        error!("{}", err);
        Html("failed").into_response()
      }
      HtmlErr::Internal(err) => {
        error!("{}", err);
        Html(err).into_response()
      }
    }
  }
}
