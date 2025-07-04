use crate::{
  cmd::{
    chat_mode::LocalSessionContext,
    check_in::{time_until, CheckInCtx},
    poll::pollstate::PollState,
  },
  config::Config,
};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::America;
use humantime::format_duration;
use kalosm::language::ChatSession;
use std::time::Duration;

// Helper function to format duration in a user-friendly way (without microseconds)
fn format_duration_clean(duration: Duration) -> String {
  let total_seconds = duration.as_secs();
  let hours = total_seconds / 3600;
  let minutes = (total_seconds % 3600) / 60;
  let seconds = total_seconds % 60;
  match (hours, minutes, seconds) {
    (h, m, s) if h > 0 && s > 0 => format!("{}h {}m {}s", h, m, s),
    (h, m, _) if h > 0 => format!("{}h {}m", h, m),
    (_, m, s) if m > 0 && s > 0 => format!("{}m {}s", m, s),
    (_, m, _) if m > 0 => format!("{}m", m),
    (_, _, s) => format!("{}s", s),
  }
}

// Helper function to truncate long IDs for display
fn truncate_id(id: &str, max_len: usize) -> String {
  if id.len() > max_len {
    format!("{}...", &id[..max_len])
  } else {
    id.to_string()
  }
}

// Helper function to generate ISO timestamp for next check-in occurrence
fn generate_iso_timestamp(now_ref: DateTime<Utc>, time: chrono::NaiveTime) -> String {
  let now_local = now_ref.with_timezone(&America::New_York);
  let target_local = America::New_York
    .from_local_datetime(&NaiveDateTime::new(now_local.date_naive(), time))
    .unwrap();

  let next_occurrence = if now_local.signed_duration_since(target_local) > chrono::Duration::zero()
  {
    // Time has passed, use tomorrow
    target_local + chrono::Duration::days(1)
  } else {
    // Time hasn't passed yet, use today
    target_local
  };

  next_occurrence
    .with_timezone(&Utc)
    .format("%Y-%m-%dT%H:%M:%SZ")
    .to_string()
}

pub fn render_admin_page(
  config: &Config,
  error: Option<&str>,
  success: Option<&str>,
  checkin_configs: Vec<(u64, CheckInCtx)>,
  active_polls: Vec<PollState>,
  chat_sessions: Vec<(String, LocalSessionContext)>,
) -> String {
  let api_key_display = if config.api_key.is_empty() {
    ""
  } else {
    "*****"
  };
  let app_id_display = if config.app_id == 0 { "0" } else { "*****" };

  let emote_users_display = config.emote_users.join(", ");

  let error_html = error
    .map(|e| format!(r#"<div class="error">❌ {}</div>"#, html_escape(e)))
    .unwrap_or_default();
  let success_html = success
    .map(|s| format!(r#"<div class="success">✅ {}</div>"#, html_escape(s)))
    .unwrap_or_default();

  // Generate CheckIn table data
  let now = Utc::now();
  let checkin_table_rows = if checkin_configs.is_empty() {
    r#"<tr><td colspan="6" class="no-data">No check-in configurations found</td></tr>"#.to_string()
  } else {
    checkin_configs
      .iter()
      .map(|(guild_id, config)| {
        let iso_timestamp = generate_iso_timestamp(now, config.poll_time);
        let time_until_duration = time_until(now, config.poll_time);
        let countdown = format_duration_clean(time_until_duration);

        // Truncate long IDs for better display
        let guild_id_display = truncate_id(&guild_id.to_string(), 10);
        let channel_id_display = truncate_id(&config.channel.to_string(), 10);

        let role_display = config
          .at_group
          .as_ref()
          .map(|role| format!("@{}", html_escape(&role.name)))
          .unwrap_or_else(|| "None".to_string());

        let duration_display = format!("{}s", config.poll_dur.as_secs());

        format!(
          r#"<tr>
            <td title="{}">{}</td>
            <td>{}</td>
            <td>{}</td>
            <td title="{}">{}</td>
            <td>{}</td>
            <td>{}</td>
          </tr>"#,
          html_escape(&guild_id.to_string()), // Full ID in tooltip
          html_escape(&guild_id_display),     // Truncated display
          html_escape(&iso_timestamp),
          html_escape(&countdown),
          html_escape(&config.channel.to_string()), // Full channel ID in tooltip
          html_escape(&channel_id_display),         // Truncated display
          html_escape(&duration_display),
          role_display
        )
      })
      .collect::<Vec<String>>()
      .join("")
  };

  // Generate Active Polls table data
  let polls_table_rows = if active_polls.is_empty() {
    r#"<tr><td colspan="4" class="no-data">No active polls found</td></tr>"#.to_string()
  } else {
    active_polls
      .iter()
      .map(|poll| {
        let end_time = poll.created_at + poll.duration;
        let end_time_formatted = DateTime::<Utc>::from(end_time)
          .format("%Y-%m-%d %H:%M:%S UTC")
          .to_string();

        let duration_display = format_duration_clean(poll.duration);
        let poll_id_display = truncate_id(&poll.id.to_string(), 12);

        // Generate voting details for the expandable section
        let voting_details = poll
          .votes
          .iter()
          .map(|(_option_key, (option_name, vote_count, voters))| {
            let voters_list = if voters.is_empty() {
              "No voters".to_string()
            } else {
              voters.iter().cloned().collect::<Vec<String>>().join(", ")
            };
            format!(
              r#"<tr>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
              </tr>"#,
              html_escape(option_name),
              vote_count,
              html_escape(&voters_list)
            )
          })
          .collect::<Vec<String>>()
          .join("");

        format!(
          r#"<tr>
            <td title="{}">{}</td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
          </tr>
          <tr class="poll-details">
            <td colspan="4">
              <details>
                <summary>View Voting Details</summary>
                <div class="poll-detail-content">
                  <table class="poll-detail-table">
                    <thead>
                      <tr>
                        <th>Option</th>
                        <th>Votes</th>
                        <th>Voters</th>
                      </tr>
                    </thead>
                    <tbody>
                      {}
                    </tbody>
                  </table>
                </div>
              </details>
            </td>
          </tr>"#,
          html_escape(&poll.id.to_string()), // Full ID in tooltip
          html_escape(&poll_id_display),     // Truncated display
          html_escape(&poll.topic),
          html_escape(&duration_display),
          html_escape(&end_time_formatted),
          voting_details
        )
      })
      .collect::<Vec<String>>()
      .join("")
  };

  // Generate Chat Sessions table data
  let chat_conversations_table_rows = if chat_sessions.is_empty() {
    r#"<tr><td colspan="4" class="no-data">No active chat sessions found</td></tr>"#.to_string()
  } else {
    chat_sessions
      .iter()
      .map(|(key, context)| {
        let session_size = context.session.history().len();

        // Calculate expiration time
        let timeout_duration = config.chat_mode_conversation_timeout;
        let expiration_time =
          context.last_activity + chrono::Duration::from_std(timeout_duration).unwrap_or_default();
        let now = chrono::Utc::now();
        let (expiration_display, time_remaining) = if expiration_time > now {
          let remaining = (expiration_time - now).to_std().unwrap_or_default();
          let remaining_formatted = format_duration_clean(remaining);
          (format!("in {}", remaining_formatted), remaining_formatted)
        } else {
          ("Expired".to_string(), "Expired".to_string())
        };

        format!(
          r#"<tr>
            <td title="{}">{}</td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
          </tr>"#,
          html_escape(key),
          html_escape(key),
          html_escape(&session_size.to_string()),
          html_escape(&time_remaining),
          html_escape(&expiration_display)
        )
      })
      .collect::<Vec<String>>()
      .join("")
  };

  // Chat mode toggle state
  let chat_mode_checked = if config.chat_mode_enabled {
    "checked"
  } else {
    ""
  };

  format!(
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>DisBot Admin Configuration</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 70%;
            margin: 0 auto;
            padding: 15px;
            background-color: #f5f5f5;
            line-height: 1.4;
        }}
        
        .container {{
            background: white;
            border-radius: 8px;
            padding: 15px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        
        .admin-layout {{
            display: flex;
            gap: 20px;
            margin-top: 20px;
        }}
        
        .left-column {{
            flex: 0 0 40%;
        }}
        
        .right-column {{
            flex: 0 0 60%;
            padding-right: 8px;
        }}
        
        @media (max-width: 768px) {{
            body {{
                max-width: 95%;
            }}
            .admin-layout {{
                flex-direction: column;
                gap: 15px;
            }}
            .left-column, .right-column {{
                flex: none;
            }}
        }}
        
        h1 {{
            color: #333;
            text-align: center;
            margin-bottom: 20px;
            border-bottom: 2px solid #007bff;
            padding-bottom: 8px;
            font-size: 22px;
        }}
        
        .form-group {{
            margin-bottom: 12px;
        }}
        
        .form-row {{
            display: flex;
            gap: 15px;
            margin-bottom: 12px;
        }}
        
        .form-row .form-group {{
            flex: 1;
            margin-bottom: 0;
        }}
        
        @media (max-width: 480px) {{
            .form-row {{
                flex-direction: column;
                gap: 8px;
            }}
        }}
        
        label {{
            display: block;
            margin-bottom: 3px;
            font-weight: 600;
            color: #555;
            font-size: 13px;
        }}
        
        input, select, textarea {{
            width: 100%;
            padding: 8px;
            border: 2px solid #ddd;
            border-radius: 4px;
            font-size: 14px;
            box-sizing: border-box;
        }}
        
        .readonly {{
            background-color: #f8f9fa;
            color: #6c757d;
            cursor: not-allowed;
            padding: 6px 8px;
            font-size: 13px;
        }}
        
        .readonly-compact {{
            background-color: #f8f9fa;
            color: #6c757d;
            cursor: not-allowed;
            padding: 4px 6px;
            font-size: 12px;
            border: 1px solid #e9ecef;
        }}
        
        input:focus, select:focus, textarea:focus {{
            outline: none;
            border-color: #007bff;
        }}
        
        .form-section {{
            border: 1px solid #e9ecef;
            border-radius: 6px;
            padding: 12px;
            margin-bottom: 15px;
        }}
        
        .form-section h3 {{
            margin-top: 0;
            margin-bottom: 10px;
            color: #495057;
            font-size: 16px;
        }}
        
        .compact-section {{
            border: 1px solid #e9ecef;
            border-radius: 6px;
            padding: 8px;
            margin-bottom: 12px;
            background-color: #f8f9fa;
        }}
        
        .compact-section h3 {{
            margin-top: 0;
            margin-bottom: 8px;
            color: #495057;
            font-size: 14px;
            font-weight: 600;
        }}
        
        .submit-btn {{
            background-color: #007bff;
            color: white;
            padding: 10px 24px;
            border: none;
            border-radius: 4px;
            font-size: 15px;
            cursor: pointer;
            width: 100%;
            margin-top: 15px;
        }}
        
        .submit-btn:hover {{
            background-color: #0056b3;
        }}
        
        .error {{
            background-color: #f8d7da;
            color: #721c24;
            padding: 12px;
            border-radius: 4px;
            margin-bottom: 20px;
            border: 1px solid #f5c6cb;
        }}
        
        .success {{
            background-color: #d4edda;
            color: #155724;
            padding: 12px;
            border-radius: 4px;
            margin-bottom: 20px;
            border: 1px solid #c3e6cb;
        }}
        
        .help-text {{
            font-size: 11px;
            color: #6c757d;
            margin-top: 2px;
            line-height: 1.3;
        }}
     
        .nav-links {{
            text-align: center;
            margin-bottom: 20px;
            padding: 10px;
            background-color: #f8f9fa;
            border-radius: 6px;
        }}
        
        .nav-links a {{
            color: #007bff;
            text-decoration: none;
            margin: 0 15px;
            font-weight: 500;
        }}
        
        .nav-links a:hover {{
            text-decoration: underline;
        }}
        
        .checkin-section {{
            border: 1px solid #e9ecef;
            border-radius: 6px;
            padding: 12px;
            margin-bottom: 15px;
            margin-right: 20px;
        }}
        
        .checkin-section h3 {{
            margin-top: 0;
            margin-bottom: 10px;
            color: #495057;
            font-size: 16px;
        }}
        
        .admin-table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 10px;
            font-size: 13px;
        }}
        
        .admin-table th,
        .admin-table td {{
            padding: 8px 6px;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }}
        
        .admin-table th {{
            background-color: #f8f9fa;
            font-weight: 600;
            color: #495057;
            font-size: 12px;
        }}
        
        .admin-table tr:hover {{
            background-color: #f8f9fa;
        }}
        
        .no-data {{
            text-align: center;
            color: #6c757d;
            font-style: italic;
            padding: 20px;
        }}
        
        .admin-table td[title] {{
            cursor: help;
        }}
        
        .section-info {{
            background-color: #e7f3ff;
            color: #0c5460;
            padding: 10px;
            border-radius: 4px;
            margin-bottom: 15px;
            border: 1px solid #b6d7ff;
            font-size: 13px;
        }}
        
        .poll-details {{
            background-color: #f8f9fa;
        }}
        
        .poll-details:hover {{
            background-color: #f8f9fa;
        }}
        
        .poll-detail-content {{
            padding: 15px;
            background-color: white;
            border-radius: 4px;
            margin-top: 10px;
        }}
        
        .poll-detail-table {{
            width: 100%;
            border-collapse: collapse;
            font-size: 12px;
        }}
        
        .poll-detail-table th,
        .poll-detail-table td {{
            padding: 8px 12px;
            text-align: left;
            border-bottom: 1px solid #e9ecef;
        }}
        
        .poll-detail-table th {{
            background-color: #f1f3f4;
            font-weight: 600;
            color: #495057;
        }}
        
        details {{
            cursor: pointer;
        }}
        
        summary {{
            font-weight: 600;
            color: #007bff;
            padding: 8px 0;
            outline: none;
        }}
        
        summary:hover {{
            color: #0056b3;
        }}
        
        @media (max-width: 768px) {{
            .admin-table {{
                font-size: 12px;
            }}
            
            .admin-table th,
            .admin-table td {{
                padding: 6px 4px;
            }}
            
            .submit-btn {{
                margin-top: 20px;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1><img src="/favicon.ico" alt="DisBot" style="width: 24px; height: 24px; vertical-align: middle; margin-right: 8px;">DisBot Admin Configuration</h1>
        
        {error_html}
        {success_html}
        
        <div class="admin-layout">
            <div class="left-column">
                <form method="post" action="/admin">
            <div class="compact-section">
                <h3>🔐 Bot Credentials</h3>
                <div class="form-row">
                    <div class="form-group">
                        <label for="api_key">API Key</label>
                        <input type="text" id="api_key" name="api_key" value="{api_key_display}" readonly class="readonly-compact">
                        <div class="help-text">Bot token - read only for security</div>
                    </div>
                    
                    <div class="form-group">
                        <label for="app_id">Application ID</label>
                        <input type="text" id="app_id" name="app_id" value="{app_id_display}" readonly class="readonly-compact">
                        <div class="help-text">Discord application ID - read only for security</div>
                    </div>
                </div>
            </div>
            
            <div class="form-section">
                <h3>⚙️ Bot Settings</h3>
                <div class="form-group">
                    <label for="emote_name">Emote Name</label>
                    <input type="text" id="emote_name" name="emote_name" value="{emote_name}" required>
                    <div class="help-text">Name for custom emoji (alphanumeric, underscore, dash only)</div>
                </div>
                
                <div class="form-group">
                    <label for="emote_users">Emote Users</label>
                    <textarea id="emote_users" name="emote_users" rows="2" placeholder="User1, User2, User3">{emote_users_display}</textarea>
                    <div class="help-text">Comma-separated list of users for emote reactions</div>
                </div>
            </div>
            
            <div class="form-section">
                <h3>📊 Logging & Performance</h3>
                <div class="form-row">
                    <div class="form-group">
                        <label for="log_level">Log Level</label>
                        <select id="log_level" name="log_level" required>
                            <option value="TRACE" {trace_selected}>TRACE</option>
                            <option value="DEBUG" {debug_selected}>DEBUG</option>
                            <option value="INFO" {info_selected}>INFO</option>
                            <option value="WARN" {warn_selected}>WARN</option>
                            <option value="ERROR" {error_selected}>ERROR</option>
                        </select>
                        <div class="help-text">✅ Takes effect immediately</div>
                    </div>
                    
                    <div class="form-group">
                        <label for="voice_channel_timeout">Voice Channel Timeout</label>
                        <input type="text" id="voice_channel_timeout" name="voice_channel_timeout" 
                               value="{timeout}" required>
                        <div class="help-text">Time before bot leaves voice channel when inactive</div>
                    </div>
                </div>
            </div>
            
            <div class="form-section">
                <h3>💬 Chat Mode Settings</h3>
                <div class="form-group">
                    <label for="chat_mode_enabled">Enable Chat Mode</label>
                    <div style="display: flex; align-items: center; margin-top: 4px;">
                        <input type="checkbox" id="chat_mode_enabled" name="chat_mode_enabled" {chat_mode_checked} style="margin-right: 8px; width: auto;">
                        <span style="font-size: 14px;">Allow users to @mention the bot for Claude AI conversations</span>
                    </div>
                </div>
            </div>
            
                    <button type="submit" class="submit-btn">💾 Save Configuration</button>
                </form>
            </div>
            
            <div class="right-column">
                <div class="checkin-section">
                    <h3>📋 Check-In Configurations</h3>
                    <div class="section-info">
                        ℹ️ Scheduled check-ins for Discord guilds. Use <code>/checkin</code> to create new ones.
                    </div>
                    
                    <table class="admin-table">
                        <thead>
                            <tr>
                                <th>Guild ID</th>
                                <th>Scheduled Time</th>
                                <th>Time Until</th>
                                <th>Channel</th>
                                <th>Duration</th>
                                <th>Role</th>
                            </tr>
                        </thead>
                        <tbody>
                            {checkin_table_rows}
                        </tbody>
                    </table>
                </div>
                
                <div class="checkin-section">
                    <h3>🗳️ Active Polls</h3>
                    <div class="section-info">
                        ℹ️ Currently active polls across Discord guilds. Expand rows to see voting details.
                    </div>
                    
                    <table class="admin-table">
                        <thead>
                            <tr>
                                <th>Poll ID</th>
                                <th>Topic</th>
                                <th>Duration</th>
                                <th>End Time</th>
                            </tr>
                        </thead>
                        <tbody>
                            {polls_table_rows}
                        </tbody>
                    </table>
                </div>
                
                <div class="checkin-section">
                    <h3>💬 Active Chat Conversations</h3>
                    <div class="section-info">
                        ℹ️ Currently active Claude AI chat conversations. Messages auto-expire after configured timeout.
                    </div>
                    
                    <table class="admin-table">
                        <thead>
                            <tr>
                                <th>Conversation Key</th>
                                <th>Messages</th>
                                <th>Time Remaining</th>
                                <th>Expires</th>
                            </tr>
                        </thead>
                        <tbody>
                            {chat_conversations_table_rows}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </div>
</body>
</html>"#,
    error_html = error_html,
    success_html = success_html,
    api_key_display = html_escape(api_key_display),
    app_id_display = html_escape(app_id_display),
    emote_name = html_escape(&config.emote_name),
    emote_users_display = html_escape(&emote_users_display),
    trace_selected = if config.log_level == "TRACE" {
      "selected"
    } else {
      ""
    },
    debug_selected = if config.log_level == "DEBUG" {
      "selected"
    } else {
      ""
    },
    info_selected = if config.log_level == "INFO" {
      "selected"
    } else {
      ""
    },
    warn_selected = if config.log_level == "WARN" {
      "selected"
    } else {
      ""
    },
    error_selected = if config.log_level == "ERROR" {
      "selected"
    } else {
      ""
    },
    timeout = format_duration(config.voice_channel_timeout),
    checkin_table_rows = checkin_table_rows,
    polls_table_rows = polls_table_rows,
    chat_conversations_table_rows = chat_conversations_table_rows,
    chat_mode_checked = chat_mode_checked,
  )
}

pub fn render_checkin_admin_page(
  checkin_configs: Vec<(u64, CheckInCtx)>,
  error: Option<&str>,
  success: Option<&str>,
) -> String {
  let error_html = error
    .map(|e| format!(r#"<div class="error">❌ {}</div>"#, html_escape(e)))
    .unwrap_or_default();
  let success_html = success
    .map(|s| format!(r#"<div class="success">✅ {}</div>"#, html_escape(s)))
    .unwrap_or_default();

  let table_rows = if checkin_configs.is_empty() {
    r#"<tr><td colspan="6" class="no-data">No check-in configurations found</td></tr>"#.to_string()
  } else {
    checkin_configs
      .iter()
      .map(|(guild_id, config)| {
        let role_display = config
          .at_group
          .as_ref()
          .map(|role| format!("@{}", html_escape(&role.name)))
          .unwrap_or_else(|| "None".to_string());

        let duration_display = format!("{}s", config.poll_dur.as_secs());

        format!(
          r#"<tr>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            <td>
              <form method="post" style="margin: 0; display: inline;">
                <input type="hidden" name="action" value="delete">
                <input type="hidden" name="guild_id" value="{}">
                <button type="submit" class="delete-btn" onclick="return confirm('Are you sure you want to delete this check-in configuration?');">Delete</button>
              </form>
            </td>
          </tr>"#,
          html_escape(&guild_id.to_string()),
          html_escape(&config.channel.to_string()),
          html_escape(&config.poll_time.to_string()),
          html_escape(&duration_display),
          role_display,
          guild_id
        )
      })
      .collect::<Vec<String>>()
      .join("")
  };

  format!(
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>DisBot CheckIn Admin</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 1000px;
            margin: 0 auto;
            padding: 15px;
            background-color: #f5f5f5;
            line-height: 1.4;
        }}
        
        .container {{
            background: white;
            border-radius: 8px;
            padding: 15px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        
        h1 {{
            color: #333;
            text-align: center;
            margin-bottom: 20px;
            border-bottom: 2px solid #007bff;
            padding-bottom: 8px;
            font-size: 22px;
        }}
        
        .nav-links {{
            text-align: center;
            margin-bottom: 20px;
            padding: 10px;
            background-color: #f8f9fa;
            border-radius: 6px;
        }}
        
        .nav-links a {{
            color: #007bff;
            text-decoration: none;
            margin: 0 15px;
            font-weight: 500;
        }}
        
        .nav-links a:hover {{
            text-decoration: underline;
        }}
        
        .error {{
            background-color: #f8d7da;
            color: #721c24;
            padding: 12px;
            border-radius: 4px;
            margin-bottom: 20px;
            border: 1px solid #f5c6cb;
        }}
        
        .success {{
            background-color: #d4edda;
            color: #155724;
            padding: 12px;
            border-radius: 4px;
            margin-bottom: 20px;
            border: 1px solid #c3e6cb;
        }}
        
        .admin-table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 15px;
        }}
        
        .admin-table th,
        .admin-table td {{
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }}
        
        .admin-table th {{
            background-color: #f8f9fa;
            font-weight: 600;
            color: #495057;
        }}
        
        .admin-table tr:hover {{
            background-color: #f8f9fa;
        }}
        
        .no-data {{
            text-align: center;
            color: #6c757d;
            font-style: italic;
            padding: 30px;
        }}
        
        .delete-btn {{
            background-color: #dc3545;
            color: white;
            padding: 6px 12px;
            border: none;
            border-radius: 4px;
            font-size: 12px;
            cursor: pointer;
        }}
        
        .delete-btn:hover {{
            background-color: #c82333;
        }}
        
        .section-info {{
            background-color: #e7f3ff;
            color: #0c5460;
            padding: 12px;
            border-radius: 4px;
            margin-bottom: 20px;
            border: 1px solid #b6d7ff;
        }}
        
        @media (max-width: 768px) {{
            .admin-table {{
                font-size: 14px;
            }}
            
            .admin-table th,
            .admin-table td {{
                padding: 8px 4px;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1><img src="/favicon.ico" alt="DisBot" style="width: 24px; height: 24px; vertical-align: middle; margin-right: 8px;">DisBot CheckIn Admin</h1>
        
        <div class="nav-links">
            <a href="/admin">← Back to Main Admin</a>
        </div>
        
        {error_html}
        {success_html}
        
        <div class="section-info">
            ℹ️ <strong>Check-in configurations</strong> manage scheduled polls for guilds. Use Discord slash commands to create new check-ins.
        </div>
        
        <table class="admin-table">
            <thead>
                <tr>
                    <th>Guild ID</th>
                    <th>Channel ID</th>
                    <th>Poll Time</th>
                    <th>Duration</th>
                    <th>Role</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody>
                {table_rows}
            </tbody>
        </table>
    </div>
</body>
</html>"#,
    error_html = error_html,
    success_html = success_html,
    table_rows = table_rows
  )
}

fn html_escape(s: &str) -> String {
  s.replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
    .replace('\'', "&#x27;")
}
