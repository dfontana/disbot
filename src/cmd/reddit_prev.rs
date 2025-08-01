use super::MessageListener;
use anyhow::{anyhow, bail};
use derive_new::new;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serenity::utils::MessageBuilder;
use serenity::{async_trait, model::channel::Message, prelude::Context};
use std::collections::HashMap;
use tracing::{info, instrument};

static REDDIT_LINK: Lazy<Regex> = Lazy::new(|| {
  Regex::new(
    r"(?x)(?i)
      (?P<url>                    # capture just url
        http[s]{0,1}://           # find http or https
        www.reddit.com            # is reddit.com
        /r/[^\s]+/                # skip over the subreddit
        comments/                 # post-id comes after this    
        (?P<postid>[a-z0-9]+)     # capture postid
        (?:/comment/)*            # maybe will be a comment
        (?P<commentid>[a-z0-9]+)? # capture comment id if it's there
      )
    ",
  )
  .unwrap()
});

#[derive(Deserialize)]
struct RedditApi {
  data: RedditData,
}

#[derive(Deserialize)]
struct RedditData {
  children: Vec<RedditKind>,
}

#[derive(Deserialize, Clone)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
enum RedditKind {
  T1(RedditComment),
  T3(RedditPost),
}

#[derive(Deserialize, Clone)]
struct RedditComment {
  link_id: String,
  author: String,
  body: String,
}

#[derive(Deserialize, Clone)]
struct RedditPost {
  title: String,
  author: String,

  // self-posts will have self-text, images have url,
  // videos have media, others have body
  body: Option<String>,
  selftext: Option<String>,
  url: Option<String>,
  secure_media: Option<RedditMedia>,
}

#[derive(Deserialize, Clone)]
struct RedditMedia {
  reddit_video: RedditVideo,
}

#[derive(Deserialize, Clone)]
struct RedditVideo {
  fallback_url: String,
}

struct Content {
  ctype: String,
  title: String,
  author: String,
  body: Option<String>,
  linked_embed: Option<String>,
}

#[derive(new)]
pub struct RedditPreviewHandler {
  http: Client,
}

impl RedditPreviewHandler {
  async fn get_api_details(&self, entity: &str) -> Result<Content, anyhow::Error> {
    let mut req: RedditApi = self
      .http
      .get(format!("https://www.reddit.com/api/info.json?id={entity}"))
      .send()
      .await?
      .json::<RedditApi>()
      .await?;

    let kind = req
      .data
      .children
      .drain(0..1)
      .next()
      .ok_or(anyhow!("No data from the Reddit API"))?;

    match kind {
      RedditKind::T1(comm) => {
        let mut parent_req = self
          .http
          .get(format!(
            "https://www.reddit.com/api/info.json?id={}",
            comm.link_id
          ))
          .send()
          .await?
          .json::<RedditApi>()
          .await?;
        let parent_kind = parent_req.data.children.drain(0..1).next();

        Ok(Content {
          ctype: "Comment".into(),
          title: parent_kind
            .and_then(|k| match k {
              RedditKind::T3(post) => Some(post.title),
              _ => None,
            })
            .unwrap_or_else(|| "(No Title)".into()),
          author: comm.author,
          body: Some(comm.body),
          linked_embed: None,
        })
      }
      RedditKind::T3(post) => {
        let ctype = {
          if post.body.as_ref().filter(|s| !s.is_empty()).is_some() {
            "Post"
          } else if post.selftext.as_ref().filter(|s| !s.is_empty()).is_some() {
            "SelfPost"
          } else if post.secure_media.is_some() {
            "Video"
          } else if post.url.as_ref().filter(|s| !s.is_empty()).is_some() {
            "Image"
          } else {
            ""
          }
        };
        let embed = {
          if ctype == "Image" || ctype == "Video" {
            post
              .url
              .or(post.secure_media.map(|sm| sm.reddit_video.fallback_url))
              .filter(|s| !s.is_empty())
          } else {
            None
          }
        };
        Ok(Content {
          ctype: ctype.into(),
          title: post.title,
          author: post.author,
          body: post.body.or(post.selftext).filter(|s| !s.is_empty()),
          linked_embed: embed,
        })
      }
    }
  }
}

fn cap_as_map(inp: &str) -> Option<HashMap<&str, &str>> {
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
  async fn message(&self, ctx: &Context, msg: &Message) -> Result<(), anyhow::Error> {
    if msg.author.id == ctx.cache.as_ref().current_user().id {
      info!("Skipping, self message");
      return Ok(());
    }

    let cap_dict = match cap_as_map(&msg.content) {
      Some(c) => c,
      None => {
        info!("No reddit link, skipping");
        return Ok(());
      }
    };

    let postid = cap_dict.get("postid").unwrap();
    let maybe_commid = cap_dict.get("commentid");
    let entity = match maybe_commid {
      Some(commid) => format!("t1_{commid}"),
      None => format!("t3_{postid}"),
    };
    let content = self.get_api_details(&entity).await?;
    if content.ctype.is_empty() || content.ctype == "Video" {
      info!("Skipping non-previewable post");
      return Ok(());
    }

    let mut bld = MessageBuilder::new();
    bld
      .push_line(format!(
        "Why I gotta do everything here... {} Summary",
        content.ctype
      ))
      .push_bold_line_safe(content.title)
      .push_italic_line_safe(format!("Shared by {}", content.author));
    if let Some(embed) = content.linked_embed {
      bld.push_line_safe(embed);
    } else if let Some(body) = content.body {
      bld.quote_rest().push_line_safe(body);
    }
    let preview = bld.to_string();

    if let Err(err) = msg.channel_id.say(&ctx.http, preview).await {
      bail!("Failed to send preview {:?}", err);
    }
    Ok(())
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
