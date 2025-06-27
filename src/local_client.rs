use crate::{config::Config, persistence::PersistentStore};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use kalosm::language::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

pub type ConversationId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSessionContext {
  pub conversation_id: ConversationId,
  pub last_activity: DateTime<Utc>,
  pub session_bytes: Vec<u8>,
}

impl LocalSessionContext {
  pub fn new(conversation_id: ConversationId, session_bytes: Vec<u8>) -> Self {
    Self {
      conversation_id,
      last_activity: Utc::now(),
      session_bytes,
    }
  }

  pub fn update_session(&mut self, session_bytes: Vec<u8>) {
    self.session_bytes = session_bytes;
    self.last_activity = Utc::now();
  }

  pub fn is_expired(&self, timeout: Duration) -> bool {
    (Utc::now() - self.last_activity)
      .to_std()
      .unwrap_or(Duration::ZERO)
      > timeout
  }
}

pub struct LocalClient {
  sessions: HashMap<ConversationId, LocalSessionContext>,
  conversation_timeout: Duration,
  persistence: Arc<PersistentStore>,
  chat_mode_enabled: bool,
  llm: Llama,
}

impl LocalClient {
  pub async fn new(config: &Config, persistence: Arc<PersistentStore>) -> Result<Self> {
    // Initialize the LLM model
    info!("Initializing local LLM model...");
    let llm = Llama::builder()
      .with_source(LlamaSource::phi_3_5_mini_4k_instruct())
      .build()
      .await
      .map_err(|e| anyhow!("Failed to initialize LLM model: {}", e))?;
    info!("Local LLM model initialized successfully");

    // Load existing sessions from persistence
    let sessions: HashMap<ConversationId, LocalSessionContext> = persistence
      .load_all_local_sessions()?
      .into_iter()
      .filter(|(_, context)| !context.is_expired(config.chat_mode_conversation_timeout))
      .collect();

    info!("Loaded {} local sessions from persistence", sessions.len());

    let mut client = Self {
      sessions,
      conversation_timeout: config.chat_mode_conversation_timeout,
      persistence,
      chat_mode_enabled: config.chat_mode_enabled,
      llm,
    };

    // Clean up any expired sessions from persistence
    client.cleanup_expired_sessions();

    Ok(client)
  }

  pub fn get_or_create_session(
    &mut self,
    conversation_key: &ConversationId,
  ) -> &mut LocalSessionContext {
    self.cleanup_expired_sessions();

    // First check if session exists in memory
    if !self.sessions.contains_key(conversation_key) {
      // Try to load from persistence
      if let Ok(Some(context)) = self.persistence.load_local_session(conversation_key) {
        if !context.is_expired(self.conversation_timeout) {
          self.sessions.insert(conversation_key.clone(), context);
        }
      }
    }

    // If still not found, create new session with empty bytes (will be initialized on first use)
    self
      .sessions
      .entry(conversation_key.clone())
      .or_insert_with(|| LocalSessionContext::new(conversation_key.clone(), Vec::new()))
  }

  pub async fn send_message_with_discord_history(
    &mut self,
    conversation_key: &ConversationId,
    discord_messages: &[(String, String)],
    current_user_message: &str,
  ) -> Result<String> {
    if !self.chat_mode_enabled {
      return Err(anyhow!("Local LLM not available - chat mode is disabled"));
    }

    // Get session bytes and ensure session exists
    let session_bytes = {
      let session_context = self.get_or_create_session(conversation_key);
      session_context.session_bytes.clone()
    };

    // Create or restore chat session
    let mut chat = if session_bytes.is_empty() {
      let mut chat = self.llm.chat().with_system_prompt(
        "You are a silly, chatty cat with sometimes snarky responses. Always respond in 3 sentences or less."
      );

      // If we have Discord history, process it to build conversation context
      if !discord_messages.is_empty() {
        // Process Discord history - only user messages for now
        // Bot messages are skipped due to kalosm API limitations
        for (author_name, content) in discord_messages {
          if !author_name.starts_with("bot:") {
            // This is a user message - add it and get a response to build conversation context
            let formatted_msg = format!("User {} says: {}", author_name, content);
            let _response = chat
              .add_message(&formatted_msg)
              .await
              .map_err(|e| anyhow!("Failed to process Discord history message: {}", e))?;
          }
          // Note: Bot messages from Discord history are currently skipped
          // This is a limitation of the kalosm API - it's designed for turn-by-turn chat
        }
      }

      chat
    } else {
      // Restore existing session
      match LlamaChatSession::from_bytes(&session_bytes) {
        Ok(session) => self.llm.chat().with_session(session),
        Err(e) => {
          warn!(
            "Failed to restore session for {}: {}. Creating new session.",
            conversation_key, e
          );
          self.llm.chat().with_system_prompt(
            "You are a silly, chatty cat with sometimes snarky responses. Always respond in 3 sentences or less."
          )
        }
      }
    };

    // Send the current user message
    let response = chat
      .add_message(current_user_message)
      .await
      .map_err(|e| anyhow!("Failed to get response from local LLM: {}", e))?;

    // Update session state and save
    match chat.session() {
      Ok(session) => {
        if let Ok(session_bytes) = session.to_bytes() {
          let session_context = self.get_or_create_session(conversation_key);
          session_context.update_session(session_bytes);
        } else {
          warn!("Failed to serialize session for {}", conversation_key);
        }
      }
      Err(e) => {
        warn!("Failed to get session for {}: {}", conversation_key, e);
      }
    }

    self.save_session_to_persistence(conversation_key);
    Ok(response)
  }

  fn save_session_to_persistence(&self, conversation_key: &ConversationId) {
    if let Some(context) = self.sessions.get(conversation_key) {
      if let Err(e) = self
        .persistence
        .save_local_session(conversation_key, context)
      {
        error!(
          "Failed to save local session {} to persistence: {}",
          conversation_key, e
        );
      }
    }
  }

  fn cleanup_expired_sessions(&mut self) {
    // Clean up memory first
    let expired_keys: Vec<ConversationId> = self
      .sessions
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
      self.sessions.remove(key);
    }

    // Also clean up expired sessions from persistence
    if let Err(e) = self
      .persistence
      .cleanup_expired_local_sessions(self.conversation_timeout)
    {
      error!(
        "Failed to cleanup expired local sessions from persistence: {}",
        e
      );
    }
  }
}
