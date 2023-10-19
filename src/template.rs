use std::{error::Error, sync::Arc};

use axum::{
  extract::{FromRef, Path, State},
  response::{Html, IntoResponse, Redirect, Response},
  routing::{get, post},
  Router,
};
use serde::Serialize;
use tera::{Context, Tera};
use tokio::sync::oneshot;
use tracing::error;
use uuid::Uuid;

use crate::{actor::ActorHandle, cmd::PollMessage};

#[derive(Clone)]
struct AppState {
  tera: Arc<Tera>,
  poll_handle: ActorHandle<PollMessage>,
}

impl FromRef<AppState> for Arc<Tera> {
  fn from_ref(app_state: &AppState) -> Arc<Tera> {
    app_state.tera.clone()
  }
}

impl FromRef<AppState> for ActorHandle<PollMessage> {
  fn from_ref(app_state: &AppState) -> ActorHandle<PollMessage> {
    app_state.poll_handle.clone()
  }
}

pub fn admin_routes(poll_handle: ActorHandle<PollMessage>) -> Router {
  let mut tera = match Tera::new("templates/**/*.html") {
    Ok(t) => t,
    Err(e) => {
      println!("Parsing error(s): {}", e);
      ::std::process::exit(1);
    }
  };
  tera.autoescape_on(vec![".html", ".sql"]);

  Router::new()
    .route("/", get(index))
    .route("/polls", get(polls))
    .route("/polls/:id", post(delete_poll))
    .with_state(AppState {
      tera: Arc::new(tera),
      poll_handle,
    })
}

#[derive(Serialize)]
struct Index<'a> {
  title: &'a str,
  pages: Vec<&'a str>,
}

async fn index(State(tera): State<Arc<Tera>>) -> Result<Html<String>, HtmlErr> {
  let index = Index {
    title: "Disbot Admin UI",
    pages: vec!["polls"],
  };
  try_render(tera, "index.html", index)
}

#[derive(Serialize)]
struct Polls<'a> {
  title: &'a str,
  polls: Vec<PollInfo>,
  poll_state_fields: Vec<&'a str>,
}

#[derive(Serialize)]
struct PollInfo {
  id: String,
  duration: String,
  topic: String,
}

async fn polls(
  State(tera): State<Arc<Tera>>,
  State(poll_handle): State<ActorHandle<PollMessage>>,
) -> Result<Html<String>, HtmlErr> {
  let (send, recv) = oneshot::channel();
  poll_handle.send(PollMessage::GetAdminState(send)).await;
  let admin_info = match recv.await {
    Err(_) => return Err(HtmlErr::Rendering("no data".into())),
    Ok(v) => v,
  };

  let polls = Polls {
    title: "Poll Controls",
    polls: admin_info
      .iter()
      .map(|e| PollInfo {
        id: e.id.to_string(),
        duration: format!("{:?}", e.duration),
        topic: e.topic.to_owned(),
      })
      .collect(),
    poll_state_fields: vec!["id", "duration", "topic"],
  };
  try_render(tera, "polls.html", polls)
}

async fn delete_poll(
  State(poll_handle): State<ActorHandle<PollMessage>>,
  Path(poll_id): Path<Uuid>,
) -> impl IntoResponse {
  poll_handle.send(PollMessage::ExpirePoll(poll_id)).await;
  Redirect::to("/ui/polls")
}

fn try_render(
  tera: Arc<Tera>,
  template: &str,
  values: impl Serialize,
) -> Result<Html<String>, HtmlErr> {
  tera
    .render(
      template,
      &Context::from_serialize(&values).map_err(|err| HtmlErr::Rendering(Box::new(err)))?,
    )
    .map_err(|err| HtmlErr::Rendering(Box::new(err)))
    .map(Html)
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
