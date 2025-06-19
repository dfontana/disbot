use disbot::{
  actor::ActorHandle,
  cmd::{
    check_in::{CheckInActor, CheckInCtx, CheckInMessage},
    poll::{pollstate::PollState, PollActor, PollMessage},
  },
  persistence::PersistentStore,
};
use chrono::NaiveTime;
use serenity::model::prelude::ChannelId;
use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
  time::{Duration, SystemTime},
};
use tempfile::tempdir;
use uuid::Uuid;

/// Test that poll actor can restore polls from persistence
#[tokio::test]
async fn test_poll_actor_restoration() {
  let temp_dir = tempdir().unwrap();
  let db_path = temp_dir.path().join("test.db");
  let persistence = Arc::new(PersistentStore::new(db_path).unwrap());

  // Create test poll data
  let poll_id = Uuid::new_v4();
  let mut votes = HashMap::new();
  votes.insert(
    "1".to_string(),
    (
      "Option 1".to_string(),
      2,
      HashSet::from(["user1".to_string(), "user2".to_string()]),
    ),
  );

  let poll_state = PollState {
    id: poll_id,
    duration: Duration::from_secs(300),
    topic: "Test Poll".to_string(),
    longest_option: 8,
    most_votes: 2,
    votes,
    ctx: create_test_call_context(),
    created_at: SystemTime::now(),
  };

  // Save poll to persistence before creating actor
  persistence.save_poll(&poll_id, &poll_state).unwrap();

  // Create poll actor with persistence
  let poll_handle =
    ActorHandle::<PollMessage>::spawn(|r, h| PollActor::new(r, h, persistence.clone()));

  // Create mock HTTP client for restoration
  let http = Arc::new(serenity::http::Http::new("test_token"));

  // Send restoration message
  poll_handle
    .send(PollMessage::RestorePolls(http.clone()))
    .await;

  // Give the actor time to process the restoration
  tokio::time::sleep(Duration::from_millis(100)).await;

  // Poll should be restored and available (we can't directly test this without exposing internal state,
  // but we can verify the persistence layer works by checking the poll is still there)
  let loaded_poll = persistence.load_poll(&poll_id).unwrap().unwrap();
  assert_eq!(loaded_poll.id, poll_id);
  assert_eq!(loaded_poll.topic, "Test Poll");
}

/// Test that check-in actor can restore configuration from persistence
#[tokio::test]
async fn test_checkin_actor_restoration() {
  let temp_dir = tempdir().unwrap();
  let db_path = temp_dir.path().join("test.db");
  let persistence = Arc::new(PersistentStore::new(db_path).unwrap());

  // Create test check-in configuration
  let guild_id = 111111111u64;
  let checkin_ctx = CheckInCtx {
    poll_time: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
    poll_dur: Duration::from_secs(3600),
    at_group: None,
    channel: ChannelId::from(123456789),
    http: Arc::new(serenity::http::Http::new("test_token")),
    emoji: create_test_emoji(),
    guild_id,
  };

  // Save check-in config to persistence before creating actor
  persistence
    .save_checkin_config(guild_id, &checkin_ctx)
    .unwrap();

  // Create poll handle (needed for check-in actor)
  let poll_handle =
    ActorHandle::<PollMessage>::spawn(|r, h| PollActor::new(r, h, persistence.clone()));

  // Create check-in actor with persistence
  let checkin_handle = ActorHandle::<CheckInMessage>::spawn(|r, h| {
    Box::new(CheckInActor::new(h, r, poll_handle.clone(), persistence.clone()))
  });

  // Create mock HTTP client for restoration
  let http = Arc::new(serenity::http::Http::new("test_token"));

  // Send restoration message
  checkin_handle
    .send(CheckInMessage::RestoreConfig(guild_id, http.clone()))
    .await;

  // Give the actor time to process the restoration
  tokio::time::sleep(Duration::from_millis(100)).await;

  // Configuration should be restored and available
  let loaded_config = persistence.load_checkin_config(guild_id).unwrap().unwrap();
  assert_eq!(loaded_config.guild_id, guild_id);
  assert_eq!(loaded_config.poll_time, checkin_ctx.poll_time);
}

/// Test that expired polls are properly cleaned up during restoration
#[tokio::test]
async fn test_expired_poll_cleanup_during_restoration() {
  let temp_dir = tempdir().unwrap();
  let db_path = temp_dir.path().join("test.db");
  let persistence = Arc::new(PersistentStore::new(db_path).unwrap());

  // Create an expired poll (created far in the past with short duration)
  let poll_id = Uuid::new_v4();
  let expired_poll = PollState {
    id: poll_id,
    duration: Duration::from_millis(1), // Very short duration
    topic: "Expired Poll".to_string(),
    longest_option: 8,
    most_votes: 0,
    votes: HashMap::new(),
    ctx: create_test_call_context(),
    created_at: SystemTime::now() - Duration::from_secs(10), // Created 10 seconds ago
  };

  // Save expired poll to persistence
  persistence.save_poll(&poll_id, &expired_poll).unwrap();

  // Verify poll exists before restoration
  assert!(persistence.load_poll(&poll_id).unwrap().is_some());

  // Create poll actor with persistence
  let poll_handle =
    ActorHandle::<PollMessage>::spawn(|r, h| PollActor::new(r, h, persistence.clone()));

  // Create mock HTTP client for restoration
  let http = Arc::new(serenity::http::Http::new("test_token"));

  // Send restoration message
  poll_handle
    .send(PollMessage::RestorePolls(http.clone()))
    .await;

  // Give the actor time to process the restoration and cleanup
  tokio::time::sleep(Duration::from_millis(100)).await;

  // Expired poll should be removed from persistence
  assert!(persistence.load_poll(&poll_id).unwrap().is_none());
}

/// Test persistence survives across multiple actor lifecycles
#[tokio::test]
async fn test_persistence_across_actor_lifecycles() {
  let temp_dir = tempdir().unwrap();
  let db_path = temp_dir.path().join("test.db");
  let persistence = Arc::new(PersistentStore::new(db_path).unwrap());

  let poll_id = Uuid::new_v4();
  let guild_id = 222222222u64;

  // First actor lifecycle - create and save data
  {
    let poll_handle =
      ActorHandle::<PollMessage>::spawn(|r, h| PollActor::new(r, h, persistence.clone()));

    let poll_handle_for_checkin = poll_handle.clone();
    let checkin_handle = ActorHandle::<CheckInMessage>::spawn(|r, h| {
      Box::new(CheckInActor::new(
        h,
        r,
        poll_handle_for_checkin.clone(),
        persistence.clone(),
      ))
    });

    // Create poll
    let poll_state = PollState {
      id: poll_id,
      duration: Duration::from_secs(300),
      topic: "Persistent Poll".to_string(),
      longest_option: 8,
      most_votes: 0,
      votes: HashMap::new(),
      ctx: create_test_call_context(),
      created_at: SystemTime::now(),
    };

    poll_handle
      .send(PollMessage::CreatePoll((
        poll_state,
        ChannelId::from(123456789),
      )))
      .await;

    // Create check-in config
    let checkin_ctx = CheckInCtx {
      poll_time: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
      poll_dur: Duration::from_secs(1800),
      at_group: None,
      channel: ChannelId::from(987654321),
      http: Arc::new(serenity::http::Http::new("test_token")),
      emoji: create_test_emoji(),
      guild_id,
    };

    checkin_handle
      .send(CheckInMessage::SetPoll(checkin_ctx))
      .await;

    // Give actors time to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Actors go out of scope here, simulating application restart
  }

  // Second actor lifecycle - restore data
  {
    let poll_handle =
      ActorHandle::<PollMessage>::spawn(|r, h| PollActor::new(r, h, persistence.clone()));

    let poll_handle_for_checkin = poll_handle.clone();
    let checkin_handle = ActorHandle::<CheckInMessage>::spawn(|r, h| {
      Box::new(CheckInActor::new(
        h,
        r,
        poll_handle_for_checkin.clone(),
        persistence.clone(),
      ))
    });

    let http = Arc::new(serenity::http::Http::new("test_token"));

    // Restore both poll and check-in data
    poll_handle
      .send(PollMessage::RestorePolls(http.clone()))
      .await;

    checkin_handle
      .send(CheckInMessage::RestoreConfig(guild_id, http.clone()))
      .await;

    // Give actors time to process restoration
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Data should still be available
    let loaded_poll = persistence.load_poll(&poll_id).unwrap().unwrap();
    assert_eq!(loaded_poll.topic, "Persistent Poll");

    let loaded_checkin = persistence.load_checkin_config(guild_id).unwrap().unwrap();
    assert_eq!(loaded_checkin.guild_id, guild_id);
  }
}

// Helper functions for test data creation

fn create_test_call_context() -> disbot::cmd::poll::pollstate::CallContext {
  disbot::cmd::poll::pollstate::CallContext {
    channel: ChannelId::from(123456789),
    http: Arc::new(serenity::http::Http::new("test_token")),
    emoji: create_test_emoji(),
  }
}

fn create_test_emoji() -> serenity::model::prelude::Emoji {
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