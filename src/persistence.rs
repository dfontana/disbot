use crate::cmd::{check_in::CheckInCtx, poll::pollstate::PollState};
use anyhow::{anyhow, Result};
use bincode::{Decode, Encode};
use redb::{Database, ReadableTable, TableDefinition};
use serenity::all::GuildId;
use std::fmt::Display;
use std::marker::PhantomData;
use std::path::Path;
use tracing::error;
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

impl Id for Uuid {
  fn id(&self) -> String {
    self.to_string()
  }

  fn from(s: String) -> Self {
    Uuid::parse_str(&s).unwrap()
  }
}

impl Id for GuildId {
  fn id(&self) -> String {
    self.to_string()
  }

  fn from(s: String) -> Self {
    s.parse()
      .map_err(|e| anyhow!("Failed to parse guild_id from key {}: {}", s, e))
      .unwrap()
  }
}

pub trait Expirable {
  fn is_expired(&self) -> bool;
}

const POLL_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("polls");
const CHECKIN_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("checkins");

impl<'a, K: Id, V: Encode> Handle<'a, K, V> {
  pub fn save(&self, key: &K, data: &V) -> Result<()> {
    let serialized = bincode::encode_to_vec(data, bincode::config::standard())?;
    let write_txn = self.db.begin_write()?;
    {
      let mut table_handle = write_txn.open_table(self.table)?;
      table_handle.insert(key.id().as_str(), serialized.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
  }
}

impl<'a, K: Id, V: Decode<()>> Handle<'a, K, V> {
  pub fn load(&self, key: &K) -> Result<Option<V>> {
    let read_txn = self.db.begin_read()?;
    let table_handle = read_txn.open_table(self.table)?;
    if let Some(data) = table_handle.get(key.id().as_str())? {
      let (item, _) = bincode::decode_from_slice(data.value(), bincode::config::standard())?;
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
      let (item, _) = bincode::decode_from_slice(value.value(), bincode::config::standard())?;
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

impl<'a, K: Id + Display, V: Expirable + Decode<()>> Handle<'a, K, V> {
  pub fn cleanup_expired(&self) -> Result<usize> {
    let items: Vec<(K, V)> = self.load_all()?;
    let mut removed_count = 0;

    for (key, item) in items {
      if item.is_expired() {
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

  pub fn check_ins<'a>(&'a self) -> Handle<'a, GuildId, CheckInCtx> {
    Handle {
      db: &self.db,
      k: PhantomData,
      v: PhantomData,
      table: CHECKIN_TABLE,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    cmd::poll::pollstate::PollState,
    types::{Chan, Guil, NaiveT, Pid},
  };
  use chrono::NaiveTime;
  use serenity::model::prelude::ChannelId;
  use std::{
    collections::{HashMap, HashSet},
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
      id: Pid(Uuid::new_v4()),
      duration: Duration::from_secs(300),
      topic: "Test Poll".to_string(),
      longest_option: 8,
      most_votes: 2,
      votes,
      created_at: SystemTime::now(),
      channel: Chan(ChannelId::from(123456789)),
      guild: Guil(<serenity::all::GuildId as From<u64>>::from(111111111)),
    }
  }

  fn create_test_checkin_ctx() -> CheckInCtx {
    CheckInCtx {
      poll_time: NaiveT(NaiveTime::from_hms_opt(20, 0, 0).unwrap()),
      poll_dur: Duration::from_secs(3600),
      at_group: None, // Simplified for testing - None is easier to handle
      channel: Chan(ChannelId::from(123456789)),
      guild: Guil(<serenity::all::GuildId as From<u64>>::from(111111111)),
    }
  }

  #[test]
  fn test_handle_save() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let poll_state = create_test_poll_state();
    let poll_id = *poll_state.id;

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
    let guild_id = *checkin_ctx.guild;

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
