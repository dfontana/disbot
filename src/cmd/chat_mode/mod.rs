mod local_client;

use super::MessageListener;
use crate::{config::Config, emoji::EmojiLookup};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use derive_new::new;
pub use local_client::*;
use serde::{Deserialize, Serialize};
use serenity::{
  all::{CacheHttp, ChannelId, ChannelType, CreateMessage, CreateThread, MessageBuilder},
  async_trait,
  model::channel::Message,
  prelude::Context,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{error, info, instrument, warn};

#[derive(new)]
pub struct ChatModeHandler {
  chat_client: Arc<Mutex<LocalClient>>,
  emoji: EmojiLookup,
}

pub type ConversationId = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalSessionContext {
  pub conversation_id: ConversationId,
  pub last_activity: DateTime<Utc>,
  pub messages: Vec<String>,
}

impl LocalSessionContext {
  pub fn new(conversation_id: ConversationId) -> Self {
    Self {
      conversation_id,
      last_activity: Utc::now(),
      messages: Vec::new(),
    }
  }

  pub fn new_with(conversation_id: ConversationId, messages: Vec<String>) -> Self {
    Self {
      conversation_id,
      last_activity: Utc::now(),
      messages,
    }
  }

  pub fn update_session(&mut self, new_messages: Vec<String>) {
    self.last_activity = Utc::now();
    self.messages.extend_from_slice(&new_messages);
  }

  pub fn is_expired(&self, timeout: Duration) -> bool {
    (Utc::now() - self.last_activity)
      .to_std()
      .unwrap_or(Duration::ZERO)
      > timeout
  }
}

fn extract_user_message(ctx: &Context, msg: &Message) -> Option<String> {
  let current_user_id = ctx.cache.current_user().id;
  let bot_mention = format!("<@{}>", current_user_id);
  let bot_mention_nick = format!("<@!{}>", current_user_id);

  // Remove bot mentions from the message content
  let content = msg
    .content
    .replace(&bot_mention, "")
    .replace(&bot_mention_nick, "")
    .trim()
    .to_string();

  Some(content).filter(|s| !s.is_empty())
}

async fn get_thread_id(ctx: &Context, msg: &Message) -> Option<ChannelId> {
  match &msg.thread {
    Some(thread) => Some(thread.id),
    None => {
      // So message is either in thread or out of thread. Check the type.
      let chan = msg
        .channel(&ctx.http())
        .await
        .ok()?
        .guild()
        .map(|gc| gc.kind)?;
      match chan {
        ChannelType::PublicThread | ChannelType::PrivateThread => Some(msg.channel_id),
        _ => None,
      }
    }
  }
}

#[async_trait]
impl MessageListener for ChatModeHandler {
  #[instrument(name = "ChatMode", level = "INFO", skip(self, ctx, msg))]
  async fn message(&self, ctx: &Context, msg: &Message) -> Result<(), anyhow::Error> {
    // Skip if message is from a bot (including ourselves)
    if msg.author.bot {
      return Ok(());
    }

    let thread_id = get_thread_id(ctx, msg).await;

    // Only process if bot is mentioned unless you're responding in thread with the bot
    let client = self.chat_client.lock().await;
    if thread_id
      .filter(|tid| !client.is_active_thread(&tid.to_string()))
      .is_some()
      && !msg.mentions_me(ctx).await.ok().unwrap_or_default()
    {
      info!("Skipping message, does not mention me or in a thread");
      return Ok(());
    }

    // Check if chat mode is enabled
    let (chat_mode_enabled, emote_name) = {
      let config = Config::global_instance().read().unwrap();
      (config.chat_mode_enabled, config.emote_name.clone())
    };

    if !chat_mode_enabled {
      let emoji = self
        .emoji
        .get(&ctx.http, &ctx.cache, msg.guild_id.unwrap())
        .await?;
      msg
        .reply(
          &ctx.http,
          MessageBuilder::new()
            .push("InternalError: Chat mode is currently disabled. ")
            .emoji(&emoji)
            .push(" Maybe try again later when I'm feeling more chatty.")
            .build(),
        )
        .await?;
      return Ok(());
    }

    info!(
      "Processing chat mode message from user: {}",
      msg.author.name
    );

    // If the message is not in a thread, create a thread && init the session against that thread id
    // if the message is in a thread, use that session
    let thread_id = match thread_id {
      Some(v) => v,
      None => {
        info!("About to create a channel b/c msg thread was empty");
        msg
          .channel_id
          .create_thread_from_message(
            &ctx.http(),
            msg.id,
            CreateThread::new(format!(
              "{} x {}",
              ctx.cache.current_user().name,
              msg.author.name
            ))
            // TODO: Likely should be customizable/align to the config value
            .auto_archive_duration(serenity::all::AutoArchiveDuration::OneHour),
          )
          .await?
          .id
      }
    };
    info!("Resolved thread {}", thread_id);
    let conversation_key = thread_id.to_string();

    // Format the current message for Claude or default one if the user
    // only mentions the bot with no additional message
    let user_message = extract_user_message(ctx, msg)
      .map(|c| format!("User {} says: {}", msg.author.name, c))
      .unwrap_or_else(|| format!("User {} says: Hello!", msg.author.name));

    // Send typing indicator to show the bot is processing
    if let Err(e) = thread_id.broadcast_typing(&ctx.http).await {
      // TODO: This only lasts for 5 seconds, which bot can be slower
      //       can I tokio::select against the add_message await && a 5sec sleep loop?
      //       Maybe I can just poll every 5 seconds until response hits?
      warn!("Failed to send typing indicator: {}", e);
    }

    // Process with chat client
    let response = {
      let mut client = self.chat_client.lock().await;

      match client.add_message(&conversation_key, &user_message).await {
        Ok(response) => response,
        Err(e) => {
          error!("Failed to get response from chat client: {}", e);
          "Sorry, I encountered an error while processing your message. Please try again later."
            .to_string()
        }
      }
    };

    // Send response back to Discord
    if let Err(e) = thread_id
      .send_message(&ctx.http, CreateMessage::new().content(response))
      .await
    {
      error!("Failed to send response: {}", e);
      let bot_nick = ctx.cache.current_user().name.clone();
      msg
        .reply(
          &ctx.http,
          format!(
            "InternalError: {} doesn't really feel like speaking that much {}",
            bot_nick, emote_name
          ),
        )
        .await
        .map(|_| ())
        .map_err(|e| anyhow!(e))
    } else {
      info!(
        "Successfully sent chat mode response to user: {}",
        msg.author.name
      );
      Ok(())
    }
  }
}
