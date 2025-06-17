# Disbot Admin Interface Implementation Plan

## Project Overview
Add a simple static admin web interface to the Rust-based Discord bot "disbot" for managing configuration at runtime over the local network. The bot currently runs on a Raspberry Pi via systemd and uses environment files for configuration.

## Requirements

### Technical Stack
- **Web Server**: Axum framework
- **Frontend**: Pure HTML forms + CSS (no JavaScript)
- **Configuration Format**: TOML file
- **Config File Location**: Same directory as binary (unless CLI argument specifies otherwise)
- **Web Server Port**: CLI parameter, default 3450

### Security & Access
- Sensitive fields (API_KEY, APP_ID) must be read-only and obfuscated with 5 asterisks
- Web interface accessible over local network only
- No authentication required (secured by network isolation)

### Configuration Management
- Replace current environment variable system with TOML-based configuration
- Use existing global `INSTANCE: Lazy<RwLock<Config>>` pattern
- All configuration changes require service restart initially (can be optimized later)
- Page refresh acceptable to see updates (no real-time updates needed)

## Current Config Structure

```rust
use once_cell::sync::Lazy;
use std::sync::RwLock;

static INSTANCE: Lazy<RwLock<Config>> = Lazy::new(|| RwLock::new(Config::default()));

#[derive(Debug, Clone)]
pub struct Config {
  pub api_key: String,           // Sensitive - read-only, obfuscated
  pub app_id: u64,              // Sensitive - read-only, obfuscated  
  pub emote_name: String,        // Configurable
  pub emote_users: Vec<String>,  // Configurable - comma-separated input
  pub env: Environment,          // Configurable - dropdown
  pub log_level: String,         // Configurable - dropdown (tracing::Level compatible)
  pub timeout: u64,             // Rename to voice_channel_timeout_seconds
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Environment {
  #[default]
  Prod,
  Dev,
}
```

## Target TOML Structure

```toml
# config.toml
api_key = "your_bot_token_here"
app_id = 123456789012345678
emote_name = "shrug_dog"
emote_users = ["User1", "User2", "User3"]
env = "prod"
log_level = "INFO"
voice_channel_timeout_seconds = 600
```

## Implementation Requirements

### Configuration Management
- **Update Flow**: Update global instance → Write TOML file → Rollback instance if file write fails
- **Concurrency**: Use existing RwLock pattern for thread safety
- **Validation**: All-or-nothing validation - reject entire form if any field is invalid
- **No Backups**: File writes happen on form submission, no backup files needed

### Form Validation Rules
- `emote_name`: Non-empty, alphanumeric + underscore/dash
- `emote_users`: Any number of comma-separated usernames (trim whitespace)
- `env`: Dropdown restricted to ["prod", "dev"]
- `log_level`: Dropdown restricted to ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] (tracing::Level compatible)
- `voice_channel_timeout_seconds`: Number input, range 10-3600 seconds

### Web Interface Design
- Single HTML page with embedded CSS
- Form sections organized logically
- Restart requirement indicators: Small icon with hover tooltip for fields requiring restart
- Error display: Show validation errors on same page after form submission
- Sensitive field display: Show "*****" for api_key and app_id
- Success feedback: Clear indication when configuration saved successfully

### Error Handling
- Display validation errors directly on webpage
- Preserve valid form data when showing errors
- First-come-first-serve for concurrent users (RwLock handles this)
- Clear error messages for file write failures

## Implementation Tasks

### 1. Update Config Struct
- Add Serde derive macros for TOML serialization
- Rename `timeout` to `voice_channel_timeout_seconds`
- Change `log_level` to use `tracing::Level` enum
- Add custom serde modules for Environment and Level enums

### 2. Configuration Management
- Implement `Config::from_toml(path: &str)` method
- Implement `Config::update_from_form()` with rollback logic
- Update global instance integration
- Add TOML file I/O with error handling

### 3. Axum Web Server
- Create HTTP server with configurable port
- Implement GET route for admin page
- Implement POST route for configuration updates
- Add form validation and error handling
- Serve static HTML with embedded CSS

### 4. HTML Interface
- Create responsive form layout
- Add field validation and error display
- Implement restart requirement indicators
- Handle sensitive field obfuscation
- Add success/error feedback

### 5. CLI Integration
- Add command line argument for config file path
- Add command line argument for web server port (default 3450)
- Update main application to use TOML configuration

## Dependencies to Add
```toml
[dependencies]
axum = "0.7"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
tokio = { version = "1.0", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs"] }
tracing = "0.1"
```

## File Structure
```
src/
├── main.rs              # Entry point with CLI args
├── config.rs            # Config struct and TOML handling
├── web/
│   ├── mod.rs          # Web server module
│   ├── handlers.rs     # Route handlers
│   └── templates.rs    # HTML template generation
└── environment.rs       # Environment enum
```

## Testing Checklist
- [ ] TOML file read/write operations
- [ ] Form validation for all field types
- [ ] Concurrent access handling
- [ ] Rollback functionality on file write failure
- [ ] Sensitive field obfuscation
- [ ] Error message display
- [ ] Configuration persistence across restarts
- [ ] CLI argument parsing
- [ ] Web server binding and accessibility