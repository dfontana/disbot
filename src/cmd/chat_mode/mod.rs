mod local_client;

use super::MessageListener;
use crate::config::Config;
use anyhow::anyhow;
use derive_new::new;
pub use local_client::*;
use serenity::{
  all::{CacheHttp, CreateMessage, CreateThread},
  async_trait,
  model::channel::Message,
  prelude::Context,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, instrument, warn};

#[derive(new)]
pub struct ChatModeHandler {
  chat_client: Arc<Mutex<LocalClient>>,
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

#[async_trait]
impl MessageListener for ChatModeHandler {
  #[instrument(name = "ChatMode", level = "INFO", skip(self, ctx, msg))]
  async fn message(&self, ctx: &Context, msg: &Message) -> Result<(), anyhow::Error> {
    // Skip if message is from a bot (including ourselves)
    if msg.author.bot {
      return Ok(());
    }

    // Only process if bot is mentioned
    // TODO: Actually, this should only be true if the message isn't in a thread that I am part of
    //       (so need to check the channel id has an active session). This way the bot can just chat back and forth
    //       naturally
    if !msg.mentions_me(ctx).await.ok().unwrap_or_default() {
      return Ok(());
    }

    // Check if chat mode is enabled
    // TODO: Remove max_messages
    let (chat_mode_enabled, emote_name, max_messages) = {
      let config = Config::global_instance().read().unwrap();
      (
        config.chat_mode_enabled,
        config.emote_name.clone(),
        config.chat_mode_max_messages_per_conversation,
      )
    };

    if !chat_mode_enabled {
      msg
        .reply(
          &ctx.http,
          format!(
        "InternalError: Chat mode is currently disabled. {} Maybe try again later when I'm feeling more chatty.",
        emote_name
      ),
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
    let thread_id = match &msg.thread {
      // Msg created a thread, reply there
      Some(thread) => thread.id,
      None => {
        // So message is either in thread or out of thread. Check the type.
        let chan = msg
          .channel(&ctx.http())
          .await?
          .guild()
          .map(|gc| gc.kind)
          .unwrap();
        match chan {
          serenity::all::ChannelType::PublicThread | serenity::all::ChannelType::PrivateThread => {
            // In thread, reply there.
            msg.channel_id
          }
          _ => {
            // Out of thread, create a thread!
            info!("About to create a channel b/c msg.thread was empty");
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
        }
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
