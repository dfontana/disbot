use std::collections::HashMap;

use super::MessageListener;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use select::predicate::{Class, Name, Predicate};
use select::{document::Document, predicate::Attr};
use serenity::utils::MessageBuilder;
use serenity::{async_trait, model::channel::Message, prelude::Context};
use tracing::{error, info, instrument, warn};

static HTTP: Lazy<Client> = Lazy::new(|| Client::new());
static REDDIT_LINK: Lazy<Regex> = Lazy::new(|| {
  Regex::new(
    r"(?x)(?i)
      (?P<url>                    # capture just url
        http[s]{0,1}://           # find http or https
        www.reddit.com            # is reddit.com
        /r/[^\s]+/                # skip over the subreddit
        comments/                 # post-id comes after this    
        (?P<postid>[a-z1-9]+)     # capture postid
        (?:/comment/)*            # maybe will be a comment
        (?P<commentid>[a-z1-9]+)? # capture comment id if it's there
      )
    ",
  )
  .unwrap()
});

#[derive(Default)]
pub struct RedditPreviewHandler {}

impl RedditPreviewHandler {
  fn get_post_title(&self, doc: &Document, postid: &str) -> Result<String, String> {
    // #t3_{post-id} h1 => .text()
    let post_box = format!("t3_{}", postid);
    doc
      .find(Attr("id", post_box.as_str()).descendant(Name("h1")))
      .next()
      .map(|node| node.text())
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

  fn get_usr(&self, doc: &Document, commentid: &str, prefix: &str) -> Result<String, String> {
    // UserInfoTooltip--t{1|3}_{comment-id}
    // TODO this doesn't work and it smells like lazy loading of the user name to the DOM.
    //      so we'll have to revisit this one at a later date
    let comm_box = format!("UserInfoTooltip--{}_{}", prefix, commentid);
    doc
      .find(Attr("id", comm_box.as_str()).descendant(Name("a")))
      .next()
      .map(|node| node.text())
      .ok_or("User could not be found".into())
  }

  fn get_comment(&self, doc: &Document, commentid: &str) -> Option<String> {
    // .Comment .t1_{comment-id} > .RichTextJSON-root
    // Loop over all children & map to their .text(), concatenate with newlines(? or maybe only ps, else it's spaces)
    let comm_box = format!("t1_{}", commentid);
    let mut comm = doc
      .find(
        Class(comm_box.as_str())
          .and(Class("Comment"))
          .descendant(Class("RichTextJSON-root")),
      )
      .next()?
      .children()
      .map(|node| node.text())
      .collect::<Vec<String>>()
      .join("\n");
    comm.truncate(200);
    Some(comm)
  }

  async fn get_document(&self, url: &str) -> Result<Document, reqwest::Error> {
    let body = HTTP.get(url).send().await?.text().await?;
    return Ok(Document::from(body.as_str()));
  }
}

fn cap_as_map(inp: &String) -> Option<HashMap<&str, &str>> {
  let caps = REDDIT_LINK.captures(inp)?;
  let cap_dict: HashMap<&str, &str> = REDDIT_LINK
    .capture_names()
    .flatten()
    .filter_map(|n| Some((n, caps.name(n)?.as_str())))
    .collect();
  Some(cap_dict)
}

#[async_trait]
impl MessageListener for RedditPreviewHandler {
  #[instrument(name = "RedditPreview", level = "INFO", skip(self, ctx, msg))]
  async fn message(&self, ctx: &Context, msg: &Message) {
    if msg.is_own(&ctx.cache) {
      info!("Skipping, self message");
      return;
    }

    let cap_dict = match cap_as_map(&msg.content) {
      Some(c) => c,
      None => {
        info!("No reddit link, skipping");
        return;
      }
    };
    let url = cap_dict.get("url").unwrap();
    let postid = cap_dict.get("postid").unwrap();

    let preview = {
      let doc = match self.get_document(url).await {
        Ok(v) => v,
        Err(err) => {
          warn!("Failed to find parse reddit post: {}", err);
          return;
        }
      };

      let maybe_title = self.get_post_title(&doc, postid);
      let maybe_image = self.get_img(&doc, postid);
      let maybe_user = match cap_dict.get("commentid") {
        Some(commid) => self.get_usr(&doc, commid, "t1"),
        None => self.get_usr(&doc, postid, "t3"),
      };
      let mut maybe_comment = None;
      let mut post_type = "Image";
      if let Some(commentid) = cap_dict.get("commentid") {
        maybe_comment = self.get_comment(&doc, commentid);
        post_type = "Comment";
      }

      if maybe_image.is_none() && maybe_comment.is_none() {
        info!("Skipping non-comment/image Reddit post");
        return;
      }

      let mut bld = MessageBuilder::new();
      bld
        .push_line(format!(
          "Why I gotta do everything here... {} Summary",
          post_type
        ))
        .push_bold_line(maybe_title.unwrap_or("".into()));
      if let Some(img) = maybe_image {
        bld.push_line(img);
      }
      if let Some(cmt) = maybe_comment {
        bld.push_quote_line(cmt);
      }
      bld.to_string()
    };

    if let Err(err) = msg.channel_id.say(&ctx.http, preview).await {
      error!("Failed to send preview {:?}", err);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::cap_as_map;
  use test_case::test_case;

  #[test_case("http://www.google.com/".to_owned(), None, None, None ; "Not Reddit")]
  #[test_case("http://www.reddit.com/r/Eldenring/comments/".to_owned(), None, None, None ; "Reddit, but no Post")]
  #[test_case(
    "https://www.reddit.com/r/Eldenring/comments/tz1ycq".to_owned(),
    Some(&"https://www.reddit.com/r/Eldenring/comments/tz1ycq"), Some(&"tz1ycq"), None;
    "Secure Post"
  )]
  #[test_case(
    "http://www.reddit.com/r/Eldenring/comments/tz1ycq".to_owned(), 
    Some(&"http://www.reddit.com/r/Eldenring/comments/tz1ycq"), Some(&"tz1ycq"), None;
    "Post"
  )]
  #[test_case(
    "http://www.reddit.com/r/Eldenring/comments/tz1ycq/".to_owned(),
    Some(&"http://www.reddit.com/r/Eldenring/comments/tz1ycq"), Some(&"tz1ycq"), None;
    "Post Trailing"
  )]
  #[test_case(
    "http://www.reddit.com/r/Eldenring/comments/tz1ycq/comment".to_owned(), 
    Some(&"http://www.reddit.com/r/Eldenring/comments/tz1ycq"), Some(&"tz1ycq"), None;
    "Partial Comment"
  )]
  #[test_case(
    "http://www.reddit.com/r/Eldenring/comments/tz1ycq/comment/".to_owned(), 
    Some(&"http://www.reddit.com/r/Eldenring/comments/tz1ycq/comment/"), Some(&"tz1ycq"), None;
    "Partial Comment Trailing"
  )]
  #[test_case(
    "http://www.reddit.com/r/Eldenring/comments/tz1ycq/comment/aTes3t".to_owned(), 
    Some(&"http://www.reddit.com/r/Eldenring/comments/tz1ycq/comment/aTes3t"), Some(&"tz1ycq"), Some(&"aTes3t");
    "Comment"
  )]
  #[test_case(
    "http://www.reddit.com/r/Eldenring/comments/tz1ycq/comment/aTes3t/".to_owned(), 
    Some(&"http://www.reddit.com/r/Eldenring/comments/tz1ycq/comment/aTes3t"), Some(&"tz1ycq"), Some(&"aTes3t");
    "Comment Trailing"
  )]
  fn verify_regex(
    url: String,
    exp_url: Option<&&str>,
    exp_postid: Option<&&str>,
    exp_cmid: Option<&&str>,
  ) {
    let mybeact = cap_as_map(&url);
    if exp_url.is_some() {
      let act = mybeact.unwrap();
      assert_eq!(act.get("url"), exp_url);
      assert_eq!(act.get("postid"), exp_postid);
      assert_eq!(act.get("commentid"), exp_cmid);
    } else {
      assert!(mybeact.is_none());
    }
  }
}
