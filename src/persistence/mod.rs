use anyhow::Result;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;
use uuid::Uuid;

use crate::cmd::{check_in::CheckInCtx, poll::pollstate::PollState};

const POLL_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("polls");
const CHECKIN_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("checkins");

pub struct PersistentStore {
  db: Database,
}

impl PersistentStore {
  pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
    let db = Database::create(path)?;

    // Initialize tables
    let write_txn = db.begin_write()?;
    {
      let _polls_table = write_txn.open_table(POLL_TABLE)?;
      let _checkins_table = write_txn.open_table(CHECKIN_TABLE)?;
    }
    write_txn.commit()?;

    Ok(PersistentStore { db })
  }

  pub fn save_poll(&self, id: &Uuid, poll_state: &PollState) -> Result<()> {
    let serialized = serde_json::to_vec(poll_state)?;
    let id_str = id.to_string();
    let write_txn = self.db.begin_write()?;
    {
      let mut table = write_txn.open_table(POLL_TABLE)?;
      table.insert(id_str.as_str(), serialized.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
  }

  pub fn _load_poll(&self, id: &Uuid) -> Result<Option<PollState>> {
    let id_str = id.to_string();
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(POLL_TABLE)?;

    if let Some(data) = table.get(id_str.as_str())? {
      let poll_state: PollState = serde_json::from_slice(data.value())?;
      Ok(Some(poll_state))
    } else {
      Ok(None)
    }
  }

  pub fn remove_poll(&self, id: &Uuid) -> Result<()> {
    let id_str = id.to_string();
    let write_txn = self.db.begin_write()?;
    {
      let mut table = write_txn.open_table(POLL_TABLE)?;
      table.remove(id_str.as_str())?;
    }
    write_txn.commit()?;
    Ok(())
  }

  pub fn load_all_polls(&self) -> Result<Vec<PollState>> {
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(POLL_TABLE)?;

    let mut polls = Vec::new();
    for entry in table.iter()? {
      let (_key, value) = entry?;
      let poll_state: PollState = serde_json::from_slice(value.value())?;
      polls.push(poll_state);
    }

    Ok(polls)
  }

  pub fn save_checkin_config(&self, guild_id: u64, config: &CheckInCtx) -> Result<()> {
    let serialized = serde_json::to_vec(config)?;
    let guild_id_str = guild_id.to_string();
    let write_txn = self.db.begin_write()?;
    {
      let mut table = write_txn.open_table(CHECKIN_TABLE)?;
      table.insert(guild_id_str.as_str(), serialized.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
  }

  pub fn load_checkin_config(&self, guild_id: u64) -> Result<Option<CheckInCtx>> {
    let guild_id_str = guild_id.to_string();
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(CHECKIN_TABLE)?;

    if let Some(data) = table.get(guild_id_str.as_str())? {
      let config: CheckInCtx = serde_json::from_slice(data.value())?;
      Ok(Some(config))
    } else {
      Ok(None)
    }
  }

  pub fn _remove_checkin_config(&self, guild_id: u64) -> Result<()> {
    let guild_id_str = guild_id.to_string();
    let write_txn = self.db.begin_write()?;
    {
      let mut table = write_txn.open_table(CHECKIN_TABLE)?;
      table.remove(guild_id_str.as_str())?;
    }
    write_txn.commit()?;
    Ok(())
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
  fn test_save_and_load_poll() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let poll_state = create_test_poll_state();
    let poll_id = poll_state.id;

    // Save poll
    store.save_poll(&poll_id, &poll_state).unwrap();

    // Load poll
    let loaded_poll = store._load_poll(&poll_id).unwrap().unwrap();

    // Verify data matches
    assert_eq!(loaded_poll.id, poll_state.id);
    assert_eq!(loaded_poll.duration, poll_state.duration);
    assert_eq!(loaded_poll.topic, poll_state.topic);
    assert_eq!(loaded_poll.longest_option, poll_state.longest_option);
    assert_eq!(loaded_poll.most_votes, poll_state.most_votes);
    assert_eq!(loaded_poll.votes, poll_state.votes);
    assert_eq!(loaded_poll.ctx.channel, poll_state.ctx.channel);
    assert_eq!(loaded_poll.ctx.emoji.name, poll_state.ctx.emoji.name);
  }

  #[test]
  fn test_load_nonexistent_poll() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let nonexistent_id = Uuid::new_v4();
    let result = store._load_poll(&nonexistent_id).unwrap();

    assert!(result.is_none());
  }

  #[test]
  fn test_remove_poll() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let poll_state = create_test_poll_state();
    let poll_id = poll_state.id;

    // Save poll
    store.save_poll(&poll_id, &poll_state).unwrap();

    // Verify it exists
    assert!(store._load_poll(&poll_id).unwrap().is_some());

    // Remove poll
    store.remove_poll(&poll_id).unwrap();

    // Verify it's gone
    assert!(store._load_poll(&poll_id).unwrap().is_none());
  }

  #[test]
  fn test_load_all_polls() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let poll1 = create_test_poll_state();
    let poll2 = create_test_poll_state();
    let poll3 = create_test_poll_state();

    // Save multiple polls
    store.save_poll(&poll1.id, &poll1).unwrap();
    store.save_poll(&poll2.id, &poll2).unwrap();
    store.save_poll(&poll3.id, &poll3).unwrap();

    // Load all polls
    let all_polls = store.load_all_polls().unwrap();

    assert_eq!(all_polls.len(), 3);

    let poll_ids: HashSet<Uuid> = all_polls.iter().map(|p| p.id).collect();
    assert!(poll_ids.contains(&poll1.id));
    assert!(poll_ids.contains(&poll2.id));
    assert!(poll_ids.contains(&poll3.id));
  }

  #[test]
  fn test_save_and_load_checkin_config() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let checkin_ctx = create_test_checkin_ctx();
    let guild_id = checkin_ctx.guild_id;

    // Save checkin config
    store.save_checkin_config(guild_id, &checkin_ctx).unwrap();

    // Load checkin config
    let loaded_config = store.load_checkin_config(guild_id).unwrap().unwrap();

    // Verify data matches
    assert_eq!(loaded_config.poll_time, checkin_ctx.poll_time);
    assert_eq!(loaded_config.poll_dur, checkin_ctx.poll_dur);
    assert_eq!(loaded_config.channel, checkin_ctx.channel);
    assert_eq!(loaded_config.guild_id, checkin_ctx.guild_id);
    assert_eq!(loaded_config.emoji.name, checkin_ctx.emoji.name);

    // Verify role matches (simplified to None for testing)
    assert_eq!(loaded_config.at_group, checkin_ctx.at_group);
  }

  #[test]
  fn test_load_nonexistent_checkin_config() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let nonexistent_guild_id = 999999999;
    let result = store.load_checkin_config(nonexistent_guild_id).unwrap();

    assert!(result.is_none());
  }

  #[test]
  fn test_remove_checkin_config() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    let checkin_ctx = create_test_checkin_ctx();
    let guild_id = checkin_ctx.guild_id;

    // Save checkin config
    store.save_checkin_config(guild_id, &checkin_ctx).unwrap();

    // Verify it exists
    assert!(store.load_checkin_config(guild_id).unwrap().is_some());

    // Remove checkin config
    store._remove_checkin_config(guild_id).unwrap();

    // Verify it's gone
    assert!(store.load_checkin_config(guild_id).unwrap().is_none());
  }

  #[test]
  fn test_multiple_operations() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = PersistentStore::new(db_path).unwrap();

    // Test both polls and checkin configs in same database
    let poll_state = create_test_poll_state();
    let checkin_ctx = create_test_checkin_ctx();

    // Save both
    store.save_poll(&poll_state.id, &poll_state).unwrap();
    store
      .save_checkin_config(checkin_ctx.guild_id, &checkin_ctx)
      .unwrap();

    // Load both
    let loaded_poll = store._load_poll(&poll_state.id).unwrap().unwrap();
    let loaded_checkin = store
      .load_checkin_config(checkin_ctx.guild_id)
      .unwrap()
      .unwrap();

    // Verify both exist and are correct
    assert_eq!(loaded_poll.id, poll_state.id);
    assert_eq!(loaded_checkin.guild_id, checkin_ctx.guild_id);

    // Remove both
    store.remove_poll(&poll_state.id).unwrap();
    store._remove_checkin_config(checkin_ctx.guild_id).unwrap();

    // Verify both are gone
    assert!(store._load_poll(&poll_state.id).unwrap().is_none());
    assert!(store
      .load_checkin_config(checkin_ctx.guild_id)
      .unwrap()
      .is_none());
  }
}
