use anyhow::Result;
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
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

  pub fn load_poll(&self, id: &Uuid) -> Result<Option<PollState>> {
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

  pub fn remove_checkin_config(&self, guild_id: u64) -> Result<()> {
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
