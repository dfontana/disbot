use std::{error::Error, time::Duration};

use crate::{cmd::SubCommandHandler, emoji::EmojiLookup};
use derive_new::new;
use rand::seq::SliceRandom;
use rand::thread_rng;
use reqwest::Client;
use serenity::{
  all::{CommandDataOption, CommandInteraction},
  async_trait,
  builder::EditInteractionResponse,
  client::Context,
  utils::MessageBuilder,
};

const IP_ECHOERS: &[&str; 3] = &[
  "https://api.ipify.org/",
  "https://api.my-ip.io/v1/ip",
  "https://ip.seeip.org/",
];

#[derive(new)]
pub struct Ip {
  http: Client,
  emoji: EmojiLookup,
}

#[async_trait]
impl SubCommandHandler for Ip {
  async fn handle(
    &self,
    ctx: &Context,
    itx: &CommandInteraction,
    _subopt: &CommandDataOption,
  ) -> Result<(), Box<dyn Error>> {
    let guild_id = match itx.guild_id {
      Some(g) => g,
      None => {
        return Err("No Guild Id on Interaction".into());
      }
    };

    let mut maybe_the_ip = None;
    let mut ip_echoers = IP_ECHOERS.clone();
    ip_echoers.shuffle(&mut thread_rng());
    for addr in ip_echoers {
      if let Ok(ip) = attempt_resolve(&self.http, addr).await {
        maybe_the_ip = Some(ip);
        break;
      }
    }
    let Some(the_ip) = maybe_the_ip else {
      itx
        .edit_response(
          &ctx.http,
          EditInteractionResponse::new().content("Could not resolve IP of server, Shibba is death"),
        )
        .await?;
      return Ok(());
    };

    let emoji = self.emoji.get(&ctx.http, &ctx.cache, guild_id).await?;
    let mut build = MessageBuilder::new();
    build
      .push_bold("Ya boi shruggin at ")
      .push_mono(the_ip)
      .push_bold(" I guess")
      .emoji(&emoji);
    itx
      .edit_response(
        &ctx.http,
        EditInteractionResponse::new().content(build.build()),
      )
      .await?;

    Ok(())
  }
}

async fn attempt_resolve(http: &Client, addr: &str) -> Result<String, reqwest::Error> {
  http
    .get(addr)
    .timeout(Duration::from_secs(3))
    .send()
    .await?
    .text()
    .await
}
