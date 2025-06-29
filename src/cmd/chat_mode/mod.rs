mod local_client;

use super::MessageListener;
use crate::config::Config;
use anyhow::anyhow;
use derive_new::new;
pub use local_client::*;
use serenity::{async_trait, model::channel::Message, prelude::Context};
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

  // TODO: When sent just the word "morp" the bot seems to think it got
  //    "morpn". Is there a newline that is sneaking through here?

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
    if !msg.mentions_me(ctx).await.ok().unwrap_or_default() {
      return Ok(());
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
      msg.reply(&ctx.http, response).await?;
      return Ok(());
    }

    // Everything is keyed on the first message's Id in the reply chain, which
    // may be this message
    // TODO: This appears to be finding a different first message if you:
    //   1. Mention the bot which makes a new session w/ your messaageId
    //   2. Stop the server, saving that key as the id
    //   3. Start the server and continue your reply chain, this creates a new session under the bot's message -- not your first one
    //      (unclear if it sees the original message or not)
    let conversation_key: ConversationId = conversation_messages
      .first()
      .map(|m| m.id.to_string())
      .unwrap_or_else(|| msg.id.to_string());

    // Format the current message for Claude or default one if the user
    // only mentions the bot with no additional message
    let user_message = extract_user_message(ctx, msg)
      .map(|c| format!("User {} says: {}", msg.author.name, c))
      .unwrap_or_else(|| format!("User {} says: Hello!", msg.author.name));

    // Send typing indicator to show the bot is processing
    if let Err(e) = msg.channel_id.broadcast_typing(&ctx.http).await {
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
