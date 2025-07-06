use crate::shutdown::ShutdownHook;
use crate::{config::Config, persistence::PersistentStore};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kalosm::language::*;
use std::collections::{hash_map::Entry, HashMap};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, instrument};

use super::{ConversationId, LocalSessionContext};

pub struct LocalClient {
  sessions: HashMap<ConversationId, (LlamaChatSession, LocalSessionContext)>,
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

    // Load existing sessions from persistence
    let sessions: HashMap<ConversationId, (LlamaChatSession, LocalSessionContext)> = persistence
      .sessions()
      .load_all()?
      .into_iter()
      .filter(|(_, context)| !context.is_expired(config.chat_mode_conversation_timeout))
      .map(|(id, ctx)| {
        let mut session = llm.chat().with_system_prompt(SYSTEM_PROMPT);
        ctx.messages.iter().for_each(|m| {
          session.add_message(m);
        });
        let sess: LlamaChatSession = session.session().unwrap().clone();
        (id, (sess, ctx))
      })
      .collect();

    Ok(Self {
      sessions,
      conversation_timeout: config.chat_mode_conversation_timeout,
      persistence,
      chat_mode_enabled: config.chat_mode_enabled,
      llm,
    })
  }

  pub fn is_active_thread(&self, id: &ConversationId) -> bool {
    self.sessions.contains_key(id)
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

    let chat = self.llm.chat();

    // Create or restore chat session
    let (mut chat, _) = match self.sessions.entry(id.clone()) {
      Entry::Occupied(e) if e.get().1.is_expired(self.conversation_timeout) => {
        e.remove_entry();
        info!("Session expired for {}, creating new session", id);
        (
          chat.with_system_prompt(SYSTEM_PROMPT),
          LocalSessionContext::new(id.clone()),
        )
      }
      Entry::Vacant(_) => {
        info!("No session found for {}, creating new session", id);
        (
          chat.with_system_prompt(SYSTEM_PROMPT),
          LocalSessionContext::new(id.clone()),
        )
      }
      Entry::Occupied(e) => {
        info!("Restored session for {}", id);
        (chat.with_session(e.get().0.clone()), e.get().1.clone())
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
      .and_modify(|(chat, ctx)| {
        *chat = session.clone();
        ctx.update_session(vec![current_user_message.to_string()])
      })
      .or_insert_with(|| {
        (
          session.clone(),
          LocalSessionContext::new_with(id.clone(), vec![current_user_message.to_string()]),
        )
      });

    Ok(response)
  }
}

#[async_trait]
impl ShutdownHook for LocalClient {
  #[instrument(name=NAME, level="INFO", skip(self))]
  async fn shutdown(&self) -> Result<(), anyhow::Error> {
    info!("Starting shutdown");
    // TODO: Is this still slow? It seems after getting a few messages in we've regressed again
    // Well 1 session was .5GB, so that's the real problem here. Storing messages and restoring that way will likely be more efficient.
    // How is it 0.5GB for a few lines of text? Who knows....
    for (id, ctx) in self.sessions.iter() {
      if let Err(e) = self.persistence.sessions().save(id, &ctx.1) {
        error!("Failed to save local session {} to persistence: {}", id, e);
      }
    }
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
