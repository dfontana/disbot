use crate::debug::Debug;
use regex::Regex;
use reqwest::Client;
use select::document::Document;
use select::predicate::Class;
use serenity::{model::channel::Message, prelude::Context};

pub struct RedditPreviewHandler {}

impl RedditPreviewHandler {
  pub fn new() -> Self {
    RedditPreviewHandler {}
  }

  async fn download_body(&self, url: &str) -> Result<String, reqwest::Error> {
    lazy_static! {
      static ref HTTP: Client = Client::new();
    }
    HTTP.get(url).send().await?.text().await
  }

  async fn send_preview(&self, img: &str, ctx: &Context, msg: &Message) -> Result<(), String> {
    let message = msg.channel_id.say(&ctx.http, img);
    tokio::try_join!(message)
      .map_err(|err| format!("Failed to send image preview! {:?}", err))
      .map(|_| ())
  }

  fn get_img_link(&self, body: String) -> Option<String> {
    let doc = Document::from(body.as_str());
    let val = doc
      .find(Class("ImageBox-image"))
      .next()?
      .parent()?
      .attr("href")?;
    Some(val.to_owned())
  }

  pub async fn message(&self, ctx: &Context, msg: &Message) {
    if msg.is_own(&ctx.cache).await {
      Debug::inst("reddit_prev").log("Skipping, self message");
      return;
    }
    lazy_static! {
      static ref REDDIT_LINK: Regex =
        Regex::new(r"(http[s]{0,1}://www\.reddit\.com[^\s]+)").unwrap();
    }
    let link = match REDDIT_LINK.captures(&msg.content) {
      Some(caps) => caps.get(1).unwrap().as_str(),
      None => {
        Debug::inst("reddit_prev").log("No reddit link, skipping");
        return;
      }
    };
    let body = match self.download_body(link).await {
      Ok(v) => v,
      Err(err) => {
        println!("Failed to get Body! {:?}", err);
        return;
      }
    };
    let img = match self.get_img_link(body) {
      Some(v) => v,
      None => {
        Debug::inst("reddit_prev").log("Failed to find image link");
        return;
      }
    };
    if let Err(err) = self.send_preview(&img, &ctx, &msg).await {
      println!("{:?}", err);
    }
  }
}
