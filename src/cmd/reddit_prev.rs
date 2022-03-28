use super::MessageListener;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use select::document::Document;
use select::predicate::Class;
use serenity::{async_trait, model::channel::Message, prelude::Context};
use tracing::{error, info, instrument, warn};

static HTTP: Lazy<Client> = Lazy::new(|| Client::new());
static REDDIT_LINK: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"(http[s]{0,1}://www\.reddit\.com[^\s]+)").unwrap());

#[derive(Default)]
pub struct RedditPreviewHandler {}

impl RedditPreviewHandler {
  async fn download_body(&self, url: &str) -> Result<String, reqwest::Error> {
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
}

#[async_trait]
impl MessageListener for RedditPreviewHandler {
  #[instrument(name = "RedditPreview", level = "INFO", skip(self, ctx, msg))]
  async fn message(&self, ctx: &Context, msg: &Message) {
    if msg.is_own(&ctx.cache).await {
      info!("Skipping, self message");
      return;
    }
    let link = match REDDIT_LINK.captures(&msg.content) {
      Some(caps) => caps.get(1).unwrap().as_str(),
      None => {
        info!("No reddit link, skipping");
        return;
      }
    };
    let body = match self.download_body(link).await {
      Ok(v) => v,
      Err(err) => {
        error!("Failed to get Body! {:?}", err);
        return;
      }
    };
    let img = match self.get_img_link(body) {
      Some(v) => v,
      None => {
        warn!("Failed to find image link");
        return;
      }
    };
    if let Err(err) = self.send_preview(&img, ctx, msg).await {
      error!("Failed to send preview {:?}", err);
    }
  }
}
