use crate::{
  cmd::{arg_util::Args, SubCommandHandler},
  emoji::EmojiLookup,
};
use anyhow::anyhow;
use derive_new::new;
use rand::rng;
use rand::seq::SliceRandom;
use reqwest::Client;
use serenity::{
  all::CommandInteraction, async_trait, builder::EditInteractionResponse, client::Context,
  utils::MessageBuilder,
};
use std::time::Duration;

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
    _args: &Args,
  ) -> Result<(), anyhow::Error> {
    let guild_id = match itx.guild_id {
      Some(g) => g,
      None => {
        return Err(anyhow!("No Guild Id on Interaction"));
      }
    };

    let mut maybe_the_ip = None;
    let mut ip_echoers = *IP_ECHOERS;
    ip_echoers.shuffle(&mut rng());
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
          EditInteractionResponse::new()
            .content("Could not resolve IP of server, Binkies is death"),
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
