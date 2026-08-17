## Change lifecycle
- Run `cargo build` the application to ensure it compiles
- Use `cargo clippy` to report any linting violations. Fix any warnings at the end of your changes.
- Use `cargo fmt` to run the formatter, always at the end of your changes
- Run `cargo test` after formatting, linting, and building

### Testing
- Tests should be co-located in the module being tested
- Never delete dev.toml or prod.toml files. Testing config generation should use a custom config path ("testing.toml").
- Only run the dev profile when testing, via `cargo run -- dev` 
- Use timeout 10 when running the application to check startup

### Stylistic Notes
- After the tracing logger is initialized don't use print statements anymore. Favor the tracing macros for logging.
- Avoid long method references by importing, keeping at least one level on the import where appropriate (for example "LevelFilter::from_level" is better than "filter::LevelFilter::from_level")
- Prefer method chaining and leveraging monadic methods, where possible to reduce nesting. For example "Result::and_then" instead of nested match arms
- Use of `anyhow::Error` for error propagation

## Architecture
Actors are used via `kitchen-sink` to handle:
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

### Deployment Architecture

**Target Platform:** Linux x86_64 via systemctl service
- SSH-based deployment with automatic service restart

**System Dependencies on Target:**
- `libopus-dev` and `ffmpeg` for audio processing
- `yt-dlp` for YouTube audio extraction
- Docker with TCP API enabled for game server management
