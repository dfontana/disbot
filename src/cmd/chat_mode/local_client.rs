use crate::shutdown::ShutdownHook;
use crate::{config::Config, persistence::PersistentStore};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kalosm::language::*;
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, instrument};

pub type ConversationId = String;

#[derive(Serialize, Deserialize)]
pub struct LocalSessionContext {
  pub conversation_id: ConversationId,
  pub last_activity: DateTime<Utc>,
  #[serde(with = "session_serde")]
  pub session: LlamaChatSession,
}

mod session_serde {
  use super::*;
  use serde::de::Error as DeError;
  use serde::ser::Error as SerError;
  use serde::{Deserialize, Deserializer, Serializer};

  pub fn serialize<S>(session: &LlamaChatSession, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    session
      .to_bytes()
      .map_err(|e| S::Error::custom(format!("Failed to serialize session: {}", e)))
      .and_then(|bytes| serializer.serialize_bytes(&bytes))
  }

  pub fn deserialize<'de, D>(deserializer: D) -> Result<LlamaChatSession, D::Error>
  where
    D: Deserializer<'de>,
  {
    let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
    LlamaChatSession::from_bytes(&bytes)
      .map_err(|e| D::Error::custom(format!("Failed to deserialize session: {}", e)))
  }
}

impl LocalSessionContext {
  pub fn new(conversation_id: ConversationId, session: LlamaChatSession) -> Self {
    Self {
      conversation_id,
      last_activity: Utc::now(),
      session,
    }
  }

  pub fn update_session(&mut self, session: LlamaChatSession) {
    self.session = session;
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

const NAME: &str = "local-llm";
const SYSTEM_PROMPT: &str = "You are a silly, chatty cat with sometimes snarky responses. Always respond in 3 sentences or less.";

impl LocalClient {
  #[instrument(name = NAME, level = "INFO", skip(config, persistence))]
  pub async fn new(config: &Config, persistence: Arc<PersistentStore>) -> Result<Self> {
    // Initialize the LLM model
    info!("Initializing local LLM model...");
    let llm = Llama::builder()
      .with_source(LlamaSource::phi_3_5_mini_4k_instruct())
      .build()
      .await
      .map_err(|e| anyhow!("Failed to initialize LLM model: {}", e))?;
    info!("Local LLM model initialized successfully");

    // TODO: This takes 13 seconds (what!). Turns out serialization is very very slow for this type.
    // Load existing sessions from persistence
    // let sessions: HashMap<ConversationId, LocalSessionContext> = persistence
    //   .sessions()
    //   .load_all()?
    //   .into_iter()
    //   .filter(|(_, context)| !context.is_expired(config.chat_mode_conversation_timeout))
    //   .collect();

    // info!("Loaded {} local sessions from persistence", sessions.len());

    Ok(Self {
      sessions: HashMap::new(),
      conversation_timeout: config.chat_mode_conversation_timeout,
      persistence,
      chat_mode_enabled: config.chat_mode_enabled,
      llm,
    })
  }

  #[instrument(name = NAME, level = "INFO", skip(self))]
  pub async fn add_message(
    &mut self,
    id: &ConversationId,
    current_user_message: &str,
  ) -> Result<String> {
    if !self.chat_mode_enabled {
      return Err(anyhow!("Local LLM not available - chat mode is disabled"));
    }

    let mut chat = self.llm.chat();

    // Create or restore chat session
    chat = match self.sessions.entry(id.clone()) {
      Entry::Occupied(e) if e.get().is_expired(self.conversation_timeout) => {
        e.remove_entry();
        info!("Session expired for {}, creating new session", id);
        chat.with_system_prompt(SYSTEM_PROMPT)
      }
      Entry::Vacant(_) => {
        info!("No session found for {}, creating new session", id);
        chat.with_system_prompt(SYSTEM_PROMPT)
      }
      Entry::Occupied(e) => {
        info!("Restored session for {}", id);
        chat.with_session(e.get().session.clone())
      }
    };

    // Send the current user message
    let response = chat
      .add_message(current_user_message)
      .await
      .map_err(|e| anyhow!("Failed to get response from local LLM: {}", e))?;

    // Update session state
    let session = chat.session().map_err(|e| anyhow!("{}", e))?;
    self
      .sessions
      .entry(id.clone())
      .and_modify(|c| c.update_session(session.clone()))
      .or_insert_with(|| LocalSessionContext::new(id.clone(), session.clone()));

    Ok(response)
  }
}

#[async_trait]
impl ShutdownHook for LocalClient {
  #[instrument(name=NAME, level="INFO", skip(self))]
  async fn shutdown(&self) -> Result<(), anyhow::Error> {
    info!("Starting shutdown");
    // TODO: This is superrr slow. Serialization cost is high.
    // for (id, ctx) in self.sessions.iter() {
    //   if let Err(e) = self.persistence.sessions().save(id, ctx) {
    //     error!("Failed to save local session {} to persistence: {}", id, e);
    //   }
    // }
    if let Err(e) = self
      .persistence
      .sessions()
      .cleanup_expired(self.conversation_timeout)
    {
      error!(
        "Failed to cleanup expired local sessions from persistence: {}",
        e
      );
    }
    info!("Shutdown complete");
    Ok(())
  }
}
