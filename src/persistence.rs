use crate::cmd::chat_mode::{ConversationId, LocalSessionContext};
use crate::cmd::{check_in::CheckInCtx, poll::pollstate::PollState};
use anyhow::{anyhow, Result};
use redb::{Database, ReadableTable, TableDefinition};
use std::fmt::Display;
use std::marker::PhantomData;
use std::path::Path;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

pub struct PersistentStore {
  db: Database,
}

pub trait Id {
  fn id(&self) -> String;
  fn from(s: String) -> Self;
}
pub struct Handle<'a, K: Id, V> {
  db: &'a Database,
  k: PhantomData<K>,
  v: PhantomData<V>,
  table: TableDefinition<'static, &'static str, &'static [u8]>,
}

pub struct SessionHandle<'a, K: Id, V> {
  db: &'a Database,
  k: PhantomData<K>,
  v: PhantomData<V>,
  table: TableDefinition<'static, &'static str, &'static [u8]>,
}

impl Id for Uuid {
  fn id(&self) -> String {
    self.to_string()
  }

  fn from(s: String) -> Self {
    Uuid::parse_str(&s).unwrap()
  }
}

impl Id for u64 {
  fn id(&self) -> String {
    self.to_string()
  }

  fn from(s: String) -> Self {
    s.parse()
      .map_err(|e| anyhow!("Failed to parse guild_id from key {}: {}", s, e))
      .unwrap()
  }
}

impl Id for ConversationId {
  fn id(&self) -> String {
    self.to_owned()
  }

  fn from(s: String) -> Self {
    s
  }
}

pub trait Expirable {
  fn is_expired(&self, timeout: Duration) -> bool;
}

impl Expirable for LocalSessionContext {
  fn is_expired(&self, timeout: Duration) -> bool {
    self.is_expired(timeout)
  }
}

const POLL_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("polls");
const CHECKIN_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("checkins");
const SESSION_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sessions");

impl<'a, K: Id, V: serde::Serialize> Handle<'a, K, V> {
  pub fn save(&self, key: &K, data: &V) -> Result<()> {
    let serialized = serde_json::to_vec(data)?;
    let write_txn = self.db.begin_write()?;
    {
      let mut table_handle = write_txn.open_table(self.table)?;
      table_handle.insert(key.id().as_str(), serialized.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
  }
}

impl<'a, K: Id, V: serde::de::DeserializeOwned> Handle<'a, K, V> {
  pub fn load(&self, key: &K) -> Result<Option<V>> {
    let read_txn = self.db.begin_read()?;
    let table_handle = read_txn.open_table(self.table)?;
    if let Some(data) = table_handle.get(key.id().as_str())? {
      let item: V = serde_json::from_slice(data.value())?;
      Ok(Some(item))
    } else {
      Ok(None)
    }
  }

  pub fn load_all(&self) -> Result<Vec<(K, V)>> {
    let read_txn = self.db.begin_read()?;
    let table_handle = read_txn.open_table(self.table)?;
    let mut items = Vec::new();

    for entry in table_handle.iter()? {
      let (key, value) = entry?;
      let key_string = key.value().to_string();
      let item: V = serde_json::from_slice(value.value())?;
      items.push((K::from(key_string), item));
    }

    Ok(items)
  }
}

impl<'a, K: Id, V> Handle<'a, K, V> {
  pub fn remove(&self, key: &K) -> Result<()> {
    let write_txn = self.db.begin_write()?;
    {
      let mut table_handle = write_txn.open_table(self.table)?;
      table_handle.remove(key.id().as_str())?;
    }
    write_txn.commit()?;
    Ok(())
  }
}

// SessionHandle implementation using bincode for sessions
impl<'a, K: Id, V: serde::Serialize> SessionHandle<'a, K, V> {
  pub fn save(&self, key: &K, data: &V) -> Result<()> {
    let serialized = bincode_new::serde::encode_to_vec(data, bincode_new::config::standard())?;
    let write_txn = self.db.begin_write()?;
    {
      let mut table_handle = write_txn.open_table(self.table)?;
      table_handle.insert(key.id().as_str(), serialized.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
  }
}

impl<'a, K: Id, V: serde::de::DeserializeOwned> SessionHandle<'a, K, V> {
  pub fn load(&self, key: &K) -> Result<Option<V>> {
    let read_txn = self.db.begin_read()?;
    let table_handle = read_txn.open_table(self.table)?;
    if let Some(data) = table_handle.get(key.id().as_str())? {
      let (item, _): (V, usize) =
        bincode_new::serde::decode_from_slice(data.value(), bincode_new::config::standard())?;
      Ok(Some(item))
    } else {
      Ok(None)
    }
  }

  pub fn load_all(&self) -> Result<Vec<(K, V)>> {
    let read_txn = self.db.begin_read()?;
    let table_handle = read_txn.open_table(self.table)?;
    let mut items = Vec::new();

    for entry in table_handle.iter()? {
      let (key, value) = entry?;
      let key_string = key.value().to_string();
      let (item, _): (V, usize) =
        bincode_new::serde::decode_from_slice(value.value(), bincode_new::config::standard())?;
      items.push((K::from(key_string), item));
    }

    Ok(items)
  }
}

impl<'a, K: Id, V> SessionHandle<'a, K, V> {
  pub fn remove(&self, key: &K) -> Result<()> {
    let write_txn = self.db.begin_write()?;
    {
      let mut table_handle = write_txn.open_table(self.table)?;
      table_handle.remove(key.id().as_str())?;
    }
    write_txn.commit()?;
    Ok(())
  }
}

impl<'a, K: Id + std::fmt::Display, V: Expirable + serde::de::DeserializeOwned + std::fmt::Debug>
  SessionHandle<'a, K, V>
{
  pub fn cleanup_expired(&self, timeout: Duration) -> Result<()> {
    let items: Vec<(K, V)> = self.load_all()?;
    info!("Looking for expired items out of {}", items.len());
    for (key, item) in items {
      info!("Checking for expiration: {} -> {:?}", key, item);
      if item.is_expired(timeout) {
        info!("Expired {}", key);
        if let Err(e) = self.remove(&key) {
          error!("Failed to remove expired item {}: {}", key, e);
        }
      }
    }
    Ok(())
  }
}

impl<'a, K: Id + Display, V: Expirable + serde::de::DeserializeOwned> Handle<'a, K, V> {
  pub fn cleanup_expired(&self, timeout: Duration) -> Result<usize> {
    let items: Vec<(K, V)> = self.load_all()?;
    let mut removed_count = 0;

    for (key, item) in items {
      if item.is_expired(timeout) {
        if let Err(e) = self.remove(&key) {
          error!("Failed to remove expired item {}: {}", key, e);
        } else {
          removed_count += 1;
        }
      }
    }

    Ok(removed_count)
  }
}

impl PersistentStore {
  pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
    let db = Database::create(path)?;
    let write_txn = db.begin_write()?;
    {
      let _polls_table = write_txn.open_table(POLL_TABLE)?;
      let _checkins_table = write_txn.open_table(CHECKIN_TABLE)?;
      let _sessions_table = write_txn.open_table(SESSION_TABLE)?;
    }
    write_txn.commit()?;

    Ok(PersistentStore { db })
  }

  pub fn polls<'a>(&'a self) -> Handle<'a, Uuid, PollState> {
    Handle {
      db: &self.db,
      k: PhantomData,
      v: PhantomData,
      table: POLL_TABLE,
    }
  }

  pub fn check_ins<'a>(&'a self) -> Handle<'a, u64, CheckInCtx> {
    Handle {
      db: &self.db,
      k: PhantomData,
      v: PhantomData,
      table: CHECKIN_TABLE,
    }
  }

  pub fn sessions<'a>(&'a self) -> SessionHandle<'a, ConversationId, LocalSessionContext> {
    SessionHandle {
      db: &self.db,
      k: PhantomData,
      v: PhantomData,
      table: SESSION_TABLE,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::cmd::poll::pollstate::{CallContext, PollState};
  use chrono::NaiveTime;
  use serenity::model::prelude::ChannelId;
  use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime},
  };
  use tempfile::tempdir;

  fn create_test_poll_state() -> PollState {
    let mut votes = HashMap::new();
    votes.insert(
      "1".to_string(),
      (
        "Option 1".to_string(),
        2,
        HashSet::from(["user1".to_string(), "user2".to_string()]),
      ),
    );
    votes.insert(
      "2".to_string(),
      (
        "Option 2".to_string(),
        1,
        HashSet::from(["user3".to_string()]),
      ),
    );

    PollState {
      id: Uuid::new_v4(),
      duration: Duration::from_secs(300),
      topic: "Test Poll".to_string(),
      longest_option: 8,
      most_votes: 2,
      votes,
      ctx: CallContext {
        channel: ChannelId::from(123456789),
        http: Arc::new(serenity::http::Http::new("test_token")),
        emoji: create_test_emoji(),
      },
      created_at: SystemTime::now(),
    }
  }

  fn create_test_checkin_ctx() -> CheckInCtx {
    CheckInCtx {
      poll_time: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
      poll_dur: Duration::from_secs(3600),
      at_group: None, // Simplified for testing - None is easier to handle
      channel: ChannelId::from(123456789),
      http: Arc::new(serenity::http::Http::new("test_token")),
      emoji: create_test_emoji(),
      guild_id: 111111111,
    }
  }

  fn create_test_emoji() -> serenity::model::prelude::Emoji {
    // Create an emoji using the builder pattern or deserialize from JSON
    // This is a workaround for non-exhaustive structs
    serde_json::from_str(
      r#"
    {
      "animated": false,
      "available": true,
      "id": "123456789012345678",
      "managed": false,
      "name": "test_emoji",
      "require_colons": true,
      "roles": [],
      "user": null
    }
    "#,
    )
    .expect("Failed to create test emoji")
  }

  #[test]
  fn test_handle_save() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let poll_state = create_test_poll_state();
    let poll_id = poll_state.id;

    // Test save() method
    store.polls().save(&poll_id, &poll_state).unwrap();

    // Verify it was saved
    let loaded_poll = store.polls().load(&poll_id).unwrap().unwrap();
    assert_eq!(loaded_poll.id, poll_state.id);
  }

  #[test]
  fn test_handle_load() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    // Test load() method with nonexistent key
    let nonexistent_id = Uuid::new_v4();
    let result = store.polls().load(&nonexistent_id).unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_handle_remove() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let checkin_ctx = create_test_checkin_ctx();
    let guild_id = checkin_ctx.guild_id;

    // Test save and remove
    store.check_ins().save(&guild_id, &checkin_ctx).unwrap();
    assert!(store.check_ins().load(&guild_id).unwrap().is_some());

    store.check_ins().remove(&guild_id).unwrap();
    assert!(store.check_ins().load(&guild_id).unwrap().is_none());
  }

  #[test]
  fn test_handle_load_all() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let poll1 = create_test_poll_state();
    let poll2 = create_test_poll_state();

    // Test load_all() method
    store.polls().save(&poll1.id, &poll1).unwrap();
    store.polls().save(&poll2.id, &poll2).unwrap();

    let all_polls = store.polls().load_all().unwrap();
    assert_eq!(all_polls.len(), 2);
  }
}
