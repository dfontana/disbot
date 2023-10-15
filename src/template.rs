use std::{error::Error, sync::{Arc, mpsc::Receiver}};

use axum::{
  extract::State,
  response::{Html, IntoResponse, Response},
  routing::get,
  Router,
};
use serde::Serialize;
use tera::{Context, Tera};
use tracing::error;

use crate::actor::ActorHandle;

pub fn admin_routes(re) -> Router {
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
    .with_state(Arc::new(tera))
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
  polls: Vec<PollInfo<'a>>,
}

#[derive(Serialize)]
struct PollInfo<'a> {
  id: &'a str,
  duration: &'a str,
  topic: &'a str,
}

async fn polls(
  State(tera): State<Arc<Tera>>,
  State(pollHandle): State<Arc<ActorHandle<PollMessage>>>,
  State(pollRecv): State<Arc<Receiver<AdminPollInfo>>>,
) -> Result<Html<String>, HtmlErr> {
  // Need to get poll handles passed into here
  // Need to have PollState sent in AdminPollInfo object
  // Need to convert PollStates to PollInfo correct
  // Need to implement the cancel routine
  pollHandle.send(PollMessage::GetAdminState).await;
  let admin_info = Some(pollRecv.recv().await) else {
    return Err(HtmlErr::Rendering("no data".into()))
  };
  
  let polls = Polls {
    title: "Poll Controls",
    polls: admin_info.states
      .map(|e| PollInfo{
        id: e.id,
        duration: e.duration,
        topic: e.topic,
      })
      .collect(),
  };
  try_render(tera, "polls.html", polls)
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
    .map(|s| Html(s))
}

enum HtmlErr {
  Rendering(Box<dyn Error>),
}

impl IntoResponse for HtmlErr {
  fn into_response(self) -> Response {
    match self {
      HtmlErr::Rendering(err) => {
        error!("{}", err);
        return Html("failed").into_response();
      }
    }
  }
}
