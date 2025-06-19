# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Change lifecycle
- Always run `cargo build` the application to ensure it compiles
- When relevant, run the the unit tests
- Use `cargo clippy` to report any linting violations. Fix any warnings at the end of your changes.
- Use `cargo fmt` to run the formatter, always at the end of your changes
- Run unit tests with "cargo test" after formatting, linting, and building

### Testing
- `cargo test` - Run all tests
- `cargo test test_name` - Run specific test
- Tests should be co-located in the module being tested
- Never delete dev.toml or prod.toml files. Testing config generation should use a custom config path ("testing.toml").
- Only run the dev profile when testing, via `cargo run -- dev` 
- Use timeout 10 when running the application to check startup

### Stylistic Notes
- After the tracing logger is initialized don't use print statements anymore. Favor the tracing macros for logging.
- Avoid long method references by importing, keeping at least one level on the import where appropriate (for example "LevelFilter::from_level" is better than "filter::LevelFilter::from_level")
- Prefer method chaining and leveraging monadic methods, where possible to reduce nesting. For example "Result::and_then" instead of nested match arms

### Documentation for Crates
- Use `docs.rs` for all external crate related documentation

## Architecture

### Core Framework
DisBot is built on **Serenity 0.12.2** for Discord API integration with **Songbird 0.5.0** for voice functionality. The bot uses a trait-based command architecture with async/await patterns throughout.

### Command System
Commands implement two main traits:
- **`AppInteractor`** - For Discord slash commands (`/play`, `/servers`, etc.)
- **`MessageListener`** - For message-based interactions (Reddit previews, shrug reactions)

Commands are organized into logical modules:
- `voice/` - Music playback, queue management, voice channel utilities
- `server/` - Game server management via Docker API
- `poll/` - Interactive voting system with actor-based state
- `check_in/` - Scheduled check-in system with role mentions

### Actor System
Custom actor implementation in `src/actor/mod.rs` handles asynchronous state management:
- **Poll actors** manage voting state and timeouts
- **Check-in actors** handle scheduled role mentions
- Uses Tokio mpsc channels for message passing

### Key Components

**Voice System (`voice/` module):**
- YouTube audio playback via yt-dlp integration
- Queue management with custom metadata storage using Songbird's `Track::new_with_data()` API
- Voice channel connection utilities with automatic disconnection handling

**Docker Integration (`docker.rs`):**
- Bollard client for Docker API communication over TCP (port 2375)
- Container lifecycle management for game servers
- Remote server status checking and control

**Configuration (`config.rs`):**
- Singleton pattern with environment-specific loading
- Supports both `prod.toml` and `dev.toml` TOML configurations
- Centralized access to Discord tokens, server details, and feature flags
- Runtime configuration updates via web interface

**Emoji Management (`emoji.rs`):**
- Cached emoji creation and retrieval (10-minute TTL)
- Automatic per-guild emoji registration
- Base64-encoded emoji image storage

### Dependencies

**Core Dependencies:**
- `serenity = "0.12.2"` - Discord API client
- `songbird = "0.5.0"` - Voice functionality (note: uses new metadata API)
- `bollard = "0.19.1"` - Docker API client
- `tokio = "1.0"` - Async runtime
- `anyhow = "1.0.86"` - Error handling

**Audio Dependencies:**
- `symphonia` - Audio codec support
- Requires system dependencies: `libopus-dev`, `ffmpeg`, `yt-dlp`

### Deployment Architecture

**Target Platform:** Linux x86_64 via systemctl service
- SSH-based deployment with automatic service restart

**System Dependencies on Target:**
- `libopus-dev` and `ffmpeg` for audio processing
- `yt-dlp` for YouTube audio extraction
- Docker with TCP API enabled for game server management

### Important Implementation Notes

**Songbird 0.5.0 API Changes:**
- Metadata storage uses `Track::new_with_data()` with `Arc<dyn Any + Send + Sync>`
- Metadata retrieval uses `TrackHandle::data::<T>()` with panic-safe error handling
- No longer uses TypeMap - custom data is stored as Arc-wrapped types

**Error Handling Patterns:**
- Comprehensive use of `anyhow::Error` for error propagation
- Graceful error responses in Discord interactions
- Panic-safe metadata retrieval with fallback values

**Security Considerations:**
- SSH key-based deployment (no password authentication)
- Sudo privileges required for server shutdown commands
- TOML file-based configuration management
- Docker TCP API exposed only on local network

### Docker Game Server Support

The bot manages game servers via Docker containers with configurations in `docker/minecraft/` and `docker/valheim/`. Server control requires Docker TCP API access over port 2375 and SSH access for power management commands.