use crate::{config::Config, persistence::PersistentStore};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

pub type ConversationId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
  pub role: String,
  pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeResponse {
  #[serde(rename = "type")]
  pub response_type: String,
  pub subtype: Option<String>,
  pub is_error: Option<bool>,
  pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
  pub messages: Vec<ClaudeMessage>,
  pub conversation_id: ConversationId,
  pub last_activity: DateTime<Utc>,
}

impl ConversationContext {
  pub fn new(conversation_id: ConversationId) -> Self {
    Self {
      messages: Vec::new(),
      conversation_id,
      last_activity: Utc::now(),
    }
  }

  pub fn add_message(&mut self, role: &str, content: &str) {
    self.messages.push(ClaudeMessage {
      role: role.to_string(),
      content: content.to_string(),
    });
    self.last_activity = Utc::now();
  }

  pub fn is_expired(&self, timeout: Duration) -> bool {
    (Utc::now() - self.last_activity)
      .to_std()
      .unwrap_or(Duration::ZERO)
      > timeout
  }
}

static CLAUDE_PATH: Lazy<Option<String>> = Lazy::new(|| {
  which::which("claude")
    .ok()
    .map(|path| path.to_string_lossy().to_string())
});

pub struct ClaudeClient {
  conversations: HashMap<ConversationId, ConversationContext>,
  conversation_timeout: Duration,
  persistence: Arc<PersistentStore>,
  chat_mode_enabled: bool,
}

impl ClaudeClient {
  pub fn new(config: &Config, persistence: Arc<PersistentStore>) -> Result<Self> {
    // Check if Claude CLI is available when chat mode is enabled
    if config.chat_mode_enabled && CLAUDE_PATH.is_none() {
      return Err(anyhow!(
        "Claude CLI not found in PATH - required when chat mode is enabled"
      ));
    }

    // Load existing conversations from persistence
    let conversations: HashMap<ConversationId, ConversationContext> = persistence
      .load_all_conversations()?
      .into_iter()
      .filter(|(_, context)| !context.is_expired(config.chat_mode_conversation_timeout))
      .collect();

    info!(
      "Loaded {} conversations from persistence",
      conversations.len()
    );

    let mut client = Self {
      conversations,
      conversation_timeout: config.chat_mode_conversation_timeout,
      persistence,
      chat_mode_enabled: config.chat_mode_enabled,
    };

    // Clean up any expired conversations from persistence
    client.cleanup_expired_conversations();

    Ok(client)
  }

  pub fn get_or_create_conversation(
    &mut self,
    conversation_key: &ConversationId,
  ) -> &mut ConversationContext {
    self.cleanup_expired_conversations();

    // First check if conversation exists in memory
    if !self.conversations.contains_key(conversation_key) {
      // Try to load from persistence
      if let Ok(Some(context)) = self.persistence.load_conversation(conversation_key) {
        if !context.is_expired(self.conversation_timeout) {
          self.conversations.insert(conversation_key.clone(), context);
        }
      }
    }

    self
      .conversations
      .entry(conversation_key.clone())
      .or_insert_with(|| ConversationContext::new(conversation_key.clone()))
  }

  pub async fn send_message(
    &mut self,
    conversation_key: &ConversationId,
    user_message: &str,
  ) -> Result<String> {
    // First, get the context and add the user message
    {
      let context = self.get_or_create_conversation(conversation_key);
      context.add_message("user", user_message);
    }

    // Clone the context data for the Claude call to avoid borrowing issues
    let context_data = {
      let context = self.get_or_create_conversation(conversation_key);
      context.clone()
    };

    let response = self.call_claude_cli(&context_data).await?;

    // Finally, add the assistant response
    {
      let context = self.get_or_create_conversation(conversation_key);
      let result = response.result.as_ref().unwrap(); // Safe due to validation above
      context.add_message("assistant", result);
    }

    // Save the updated conversation to persistence
    self.save_conversation_to_persistence(conversation_key);

    Ok(response.result.unwrap()) // Safe due to validation above
  }

  pub async fn send_message_with_discord_history(
    &mut self,
    conversation_key: &ConversationId,
    discord_messages: &[(String, String)], // (author_name, content) pairs
    current_user_message: &str,
  ) -> Result<String> {
    // If no Discord history, use the simple send_message method
    if discord_messages.is_empty() {
      return self
        .send_message(conversation_key, current_user_message)
        .await;
    }

    // Get or create conversation context and initialize with Discord history if new
    {
      let context = self.get_or_create_conversation(conversation_key);

      // If this is a new conversation, initialize it with Discord history
      if context.messages.is_empty() {
        for (author_name, content) in discord_messages {
          if author_name.starts_with("bot:") {
            context.add_message("assistant", content);
          } else {
            context.add_message("user", &format!("User {} says: {}", author_name, content));
          }
        }
      }

      // Add the current message
      context.add_message("user", current_user_message);
    }

    // Clone the context data for the Claude call
    let context_data = {
      let context = self.get_or_create_conversation(conversation_key);
      context.clone()
    };

    let response = self.call_claude_cli(&context_data).await?;

    // Add the assistant response
    {
      let context = self.get_or_create_conversation(conversation_key);
      let result = response.result.as_ref().unwrap(); // Safe due to validation above
      context.add_message("assistant", result);
    }

    // Save the updated conversation to persistence
    self.save_conversation_to_persistence(conversation_key);

    Ok(response.result.unwrap()) // Safe due to validation above
  }

  fn save_conversation_to_persistence(&self, conversation_key: &ConversationId) {
    if let Some(context) = self.conversations.get(conversation_key) {
      if let Err(e) = self
        .persistence
        .save_conversation(conversation_key, context)
      {
        error!(
          "Failed to save conversation {} to persistence: {}",
          conversation_key, e
        );
      }
    }
  }

  async fn call_claude_cli(&self, context: &ConversationContext) -> Result<ClaudeResponse> {
    if !self.chat_mode_enabled {
      return Err(anyhow!("Claude CLI not available - chat mode is disabled"));
    }

    let claude_path = CLAUDE_PATH
      .as_ref()
      .ok_or_else(|| anyhow!("Claude CLI not found in PATH"))?;

    let mut conversation_text = String::new();

    for message in &context.messages {
      conversation_text.push_str(&format!("{}: {}\n\n", message.role, message.content));
    }

    let system_prompt = "You are a silly, chatty cat with sometimes snarky responses. Always respond in 3 sentences or less.";

    let mut child = Command::new(claude_path)
      .args([
        "--system-prompt",
        system_prompt,
        "--print",
        "--output-format",
        "json",
        &conversation_text,
      ])
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .map_err(|e| anyhow!("Failed to spawn Claude CLI process: {}", e))?;

    let stdout = child
      .stdout
      .take()
      .ok_or_else(|| anyhow!("Failed to capture stdout"))?;

    let stderr = child
      .stderr
      .take()
      .ok_or_else(|| anyhow!("Failed to capture stderr"))?;

    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();

    let (exit_status, (), ()) = tokio::try_join!(
      child.wait(),
      async {
        let mut stdout_reader = tokio::io::BufReader::new(stdout);
        stdout_reader.read_to_end(&mut stdout_data).await?;
        Ok::<(), std::io::Error>(())
      },
      async {
        let mut stderr_reader = tokio::io::BufReader::new(stderr);
        stderr_reader.read_to_end(&mut stderr_data).await?;
        Ok::<(), std::io::Error>(())
      }
    )
    .map_err(|e| anyhow!("Failed to execute Claude CLI: {}", e))?;

    if !exit_status.success() {
      let stderr_str = String::from_utf8_lossy(&stderr_data);
      error!("Claude CLI failed with stderr: {}", stderr_str);
      return Err(anyhow!("Claude CLI exited with error: {}", stderr_str));
    }

    let stdout_str = String::from_utf8_lossy(&stdout_data);

    if stdout_str.trim().is_empty() {
      return Err(anyhow!("Claude CLI returned empty response"));
    }

    let response = match serde_json::from_str::<ClaudeResponse>(&stdout_str) {
      Ok(response) => response,
      Err(e) => {
        warn!(
          "Claude CLI returned non-JSON response: {}, stdout: {}",
          e, &stdout_str
        );
        return Err(anyhow!("Failed to parse Claude CLI JSON response: {}", e));
      }
    };

    // Validate response type
    if response.response_type != "result" {
      debug!(
        "Claude CLI returned unexpected response type: {}, expected 'result'",
        response.response_type
      );
      return Err(anyhow!(
        "Unexpected response type: {}, expected 'result'",
        response.response_type
      ));
    }

    // Check for error conditions
    if response.is_error.unwrap_or(false) {
      if let Some(ref result) = response.result {
        error!("Claude CLI returned error: {}", result);
        return Err(anyhow!("Claude CLI error: {}", result));
      } else {
        error!("Claude CLI returned error with no result message");
        return Err(anyhow!("Claude CLI returned error with no result message"));
      }
    }

    // Ensure we have a result for successful responses
    if response.result.is_none() {
      return Err(anyhow!(
        "Claude CLI returned success response with no result"
      ));
    }

    Ok(response)
  }

  fn cleanup_expired_conversations(&mut self) {
    // Clean up memory first
    let expired_keys: Vec<ConversationId> = self
      .conversations
      .iter()
      .filter_map(|(key, context)| {
        if context.is_expired(self.conversation_timeout) {
          Some(key.clone())
        } else {
          None
        }
      })
      .collect();

    for key in &expired_keys {
      self.conversations.remove(key);
    }

    // Also clean up expired conversations from persistence
    if let Err(e) = self
      .persistence
      .cleanup_expired_conversations(self.conversation_timeout)
    {
      error!(
        "Failed to cleanup expired conversations from persistence: {}",
        e
      );
    }
  }
}
