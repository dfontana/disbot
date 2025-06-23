use crate::{
  claude_client::{ClaudeClient, ConversationId},
  config::Config,
  persistence::PersistentStore,
};
use serenity::{async_trait, model::channel::Message, prelude::Context};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, instrument, warn};

use super::MessageListener;

pub struct ChatModeHandler {
  claude_client: Arc<Mutex<ClaudeClient>>,
}

impl ChatModeHandler {
  pub fn new(config: &Config, persistence: Arc<PersistentStore>) -> Self {
    let claude_client = Arc::new(Mutex::new(ClaudeClient::new(config, persistence).expect(
      "Failed to create Claude client - check configuration and Claude CLI availability",
    )));

    Self { claude_client }
  }

  fn extract_user_message(&self, ctx: &Context, msg: &Message) -> Option<String> {
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

  async fn send_response(
    &self,
    ctx: &Context,
    msg: &Message,
    response: &str,
  ) -> Result<(), String> {
    if let Err(e) = msg.reply(&ctx.http, response).await {
      error!("Failed to send response: {}", e);
      let (emote_name, bot_nick) = {
        let config = Config::global_instance().read().unwrap();
        (
          config.emote_name.clone(),
          ctx.cache.current_user().name.clone(),
        )
      };
      msg
        .reply(
          &ctx.http,
          format!(
            "InternalError: {} doesn't really feel like speaking that much {}",
            bot_nick, emote_name
          ),
        )
        .await
        .map_err(|e| format!("Failed to send snarky response: {}", e))?;
    }

    Ok(())
  }
}

#[async_trait]
impl MessageListener for ChatModeHandler {
  #[instrument(name = "ChatMode", level = "INFO", skip(self, ctx, msg))]
  async fn message(&self, ctx: &Context, msg: &Message) {
    // Skip if message is from a bot (including ourselves)
    if msg.author.bot {
      return;
    }

    // Only process if bot is mentioned
    if !msg.mentions_me(ctx).await.ok().unwrap_or_default() {
      return;
    }

    // Check if chat mode is enabled
    let (chat_mode_enabled, emote_name, max_messages) = {
      let config = Config::global_instance().read().unwrap();
      (
        config.chat_mode_enabled,
        config.emote_name.clone(),
        config.chat_mode_max_messages_per_conversation,
      )
    };

    if !chat_mode_enabled {
      if let Err(e) = msg
        .reply(
          &ctx.http,
          format!(
        "InternalError: Chat mode is currently disabled. {} Maybe try again later when I'm feeling more chatty.",
        emote_name
      ),
        )
        .await
      {
        error!("Failed to send disabled chat mode response: {}", e);
      }
      return;
    }

    info!(
      "Processing chat mode message from user: {}",
      msg.author.name
    );

    // Build conversation chain
    let conversation_messages: Vec<Message> = std::iter::once(msg)
      .flat_map(|m| m.referenced_message.iter())
      .map(|m| (**m).clone())
      // Drop any internal errors in the chain as that doesn't count
      .filter(|m| !m.content.starts_with("InternalError"))
      .take(max_messages)
      .collect::<Vec<_>>()
      .into_iter()
      .rev()
      .collect();

    // Check if we've hit the max conversation limit
    if conversation_messages.len() >= max_messages {
      let response = "Max chat length reached, start a new message chain";
      if let Err(e) = msg.reply(&ctx.http, response).await {
        error!("Failed to send max length response: {}", e);
      }
      return;
    }

    // Everythign is keyed on the first message's Id in the reply chain, which
    // mayb be this message
    let conversation_key: ConversationId = conversation_messages
      .first()
      .map(|m| m.id.to_string())
      .unwrap_or_else(|| msg.id.to_string());

    // Convert Discord messages to Claude history
    let bot_id = ctx.cache.current_user().id;
    let discord_history: Vec<(String, String)> = conversation_messages
      .iter()
      .filter_map(|msg| {
        if msg.author.id == bot_id {
          Some(msg.content.trim())
            .filter(|s| !s.is_empty())
            .map(|s| (format!("bot:{}", msg.author.name), s.to_string()))
        } else {
          self
            .extract_user_message(ctx, msg)
            .map(|c| (msg.author.name.clone(), c))
        }
      })
      .collect();

    // Format the current message for Claude or default one if the user
    // only mentions the bot with no additional message
    let user_message = self
      .extract_user_message(ctx, msg)
      .map(|c| format!("User {} says: {}", msg.author.name, c))
      .unwrap_or_else(|| format!("User {} says: Hello!", msg.author.name));

    // Send typing indicator to show the bot is processing
    if let Err(e) = msg.channel_id.broadcast_typing(&ctx.http).await {
      warn!("Failed to send typing indicator: {}", e);
    }

    // Process with Claude using Discord conversation history
    let response = {
      let mut client = self.claude_client.lock().await;

      match client
        .send_message_with_discord_history(&conversation_key, &discord_history, &user_message)
        .await
      {
        Ok(response) => response,
        Err(e) => {
          error!("Failed to get response from Claude: {}", e);
          "Sorry, I encountered an error while processing your message. Please try again later."
            .to_string()
        }
      }
    };

    // Send response back to Discord
    if let Err(e) = self.send_response(ctx, msg, &response).await {
      error!("Failed to send response to Discord: {}", e);

      // Try to send a simple error message
      if let Err(fallback_err) = msg
        .reply(
          &ctx.http,
          "InternalError: Sorry, I couldn't send my response. Please try again.",
        )
        .await
      {
        error!("Failed to send fallback error message: {}", fallback_err);
      }
    } else {
      info!(
        "Successfully sent chat mode response to user: {}",
        msg.author.name
      );
    }
  }
}
