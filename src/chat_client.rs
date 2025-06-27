use crate::{
  claude_client::{ClaudeClient, ConversationId},
  config::Config,
  local_client::LocalClient,
  persistence::PersistentStore,
  ChatClientType,
};
use anyhow::Result;
use std::sync::Arc;

pub enum AnyClient {
  Claude(ClaudeClient),
  Local(LocalClient),
}

impl AnyClient {
  pub async fn send_message_with_discord_history(
    &mut self,
    conversation_key: &ConversationId,
    discord_messages: &[(String, String)],
    current_user_message: &str,
  ) -> Result<String> {
    match self {
      AnyClient::Claude(client) => {
        client
          .send_message_with_discord_history(
            conversation_key,
            discord_messages,
            current_user_message,
          )
          .await
      }
      AnyClient::Local(client) => {
        client
          .send_message_with_discord_history(
            conversation_key,
            discord_messages,
            current_user_message,
          )
          .await
      }
    }
  }
}

pub async fn create_chat_client(
  client_type: ChatClientType,
  config: &Config,
  persistence: Arc<PersistentStore>,
) -> Result<AnyClient> {
  match client_type {
    ChatClientType::Claude => {
      let client = ClaudeClient::new(config, persistence)?;
      Ok(AnyClient::Claude(client))
    }
    ChatClientType::Local => {
      let client = LocalClient::new(config, persistence).await?;
      Ok(AnyClient::Local(client))
    }
  }
}
