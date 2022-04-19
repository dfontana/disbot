use std::collections::HashMap;

use super::MessageListener;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use select::predicate::{Class, Name, Predicate};
use select::{document::Document, predicate::Attr};
use serenity::{async_trait, model::channel::Message, prelude::Context};
use tracing::{error, info, instrument, warn};

static HTTP: Lazy<Client> = Lazy::new(|| Client::new());
static REDDIT_LINK: Lazy<Regex> = Lazy::new(|| {
  Regex::new(
    r"
  (?x)(?i)             # Comment + any-case mode
  (?P<url>             # capture just url
    http[s]{0,1}:\/\/  # find http or https
    www\.reddit\.com   # is reddit.com
    \/r\/[^\s]+\/      # skip over the subreddit
    comments\/         # post-id comes after this
    (?P<postid>        # capture post-id
      (?:(?!\/|$).)*   # any non-slash or EOL
    )
    (?:                # may be a comment link
      \/comment\/      # if so, comment is next
      (?P<commentid>   # capture comment-id
        (?:(?!\/|$).)* # any non-slash or EOL
      )
    )?                 # (Optional)
  )
",
  )
  .unwrap()
});

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

  fn get_post_title(&self, doc: &Document, postid: &str) -> Result<String, String> {
    // #t3_{post-id} h1 => .text()
    let post_box = format!("t3_{}", postid);
    doc
      .find(Attr("id", post_box.as_str()).descendant(Name("h1")))
      .next()
      .map(|node| node.as_text().unwrap().to_owned())
      .ok_or("No Post title found".into())
  }

  fn get_img(&self, doc: &Document, postid: &str) -> Option<String> {
    // #t3_{post-id} ImageBox-image if exists grab parent href for image
    let post_box = format!("t3_{}", postid);
    Some(
      doc
        .find(Attr("id", post_box.as_str()).descendant(Class("ImageBox-image")))
        .next()?
        .parent()?
        .attr("href")?
        .to_owned(),
    )
  }

  fn get_usr(&self, doc: &Document, commentid: &str) -> Option<String> {
    // UserInfoTooltip--t1_{comment-id}
    let comm_box = format!("UserInfoTooltip--t1_{}", commentid);
    Some(
      doc
        .find(Attr("id", comm_box.as_str()).descendant(Name("a")))
        .next()?
        .as_text()?
        .to_owned(),
    )
  }

  fn get_comment(&self, doc: &Document, commentid: &str) -> Option<String> {
    // .Comment .t1_{comment-id} > .RichTextJSON-root
    // Loop over all children & map to their .text(), concatenate with newlines(? or maybe only ps, else it's spaces)
    let comm_box = format!("t1_{}", commentid);
    Some(
      doc
        .find(
          Class(comm_box.as_str())
            .and(Class("Comment"))
            .descendant(Class("RichTextJSON-root")),
        )
        .next()?
        .children()
        .map(|node| node.as_text().unwrap())
        .collect::<Vec<&str>>()
        .join("\n"),
    )
  }

  async fn get_document(&self, url: &str) -> Result<Document, reqwest::Error> {
    let body = self.download_body(url).await?;
    return Ok(Document::from(body.as_str()));
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
    let caps = match REDDIT_LINK.captures(&msg.content) {
      Some(c) => c,
      None => {
        info!("No reddit link, skipping");
        return;
      }
    };
    let cap_dict: HashMap<&str, &str> = REDDIT_LINK
      .capture_names()
      .flatten()
      .filter_map(|n| Some((n, caps.name(n)?.as_str())))
      .collect();

    let url = cap_dict.get("url").unwrap();
    let doc = match self.get_document(url).await {
      Ok(v) => v,
      Err(err) => {
        warn!("Failed to find parse reddit post: {}", err);
        return;
      }
    };

    let postid = cap_dict.get("postid").unwrap();
    let maybe_title = self.get_post_title(&doc, postid);
    let maybe_image = self.get_img(&doc, postid);
    let mut maybe_user = None;
    let mut maybe_comment = None;
    if let Some(commentid) = cap_dict.get("commentid") {
      maybe_user = self.get_usr(&doc, commentid);
      maybe_comment = self.get_comment(&doc, commentid);
    }

    /*
    TODO see Test Discord for examples (Pinned)
      A) Grab post Title
      B) Maybe Grab Post Content (img) or ignore if not present
      C) Maybe Grab Commenter or ignore if not present
      D) Maybe Grab Comment

    TODO
      - Figure out how to construct the final message
      - Maybe snag video posts too (first frame is sent client side)
      - Check that comments on multi-line are handled right
      - May want to truncate if super long text post

    TODO (IDE)
      - Format shortcut is broken :thinkies: Is formatter not registered right anymore
    */

    // if let Err(err) = self.send_preview(&img, ctx, msg).await {
    //   error!("Failed to send preview {:?}", err);
    // }
  }
}
