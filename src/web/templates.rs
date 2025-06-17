use crate::config::Config;
use crate::env::Environment;

pub fn render_admin_page(config: &Config, error: Option<&str>, success: Option<&str>) -> String {
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
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f5f5f5;
            line-height: 1.6;
        }}
        
        .container {{
            background: white;
            border-radius: 8px;
            padding: 30px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        
        h1 {{
            color: #333;
            text-align: center;
            margin-bottom: 30px;
            border-bottom: 2px solid #007bff;
            padding-bottom: 10px;
        }}
        
        .form-group {{
            margin-bottom: 20px;
        }}
        
        label {{
            display: block;
            margin-bottom: 5px;
            font-weight: 600;
            color: #555;
        }}
        
        input, select, textarea {{
            width: 100%;
            padding: 10px;
            border: 2px solid #ddd;
            border-radius: 4px;
            font-size: 14px;
            box-sizing: border-box;
        }}
        
        input:focus, select:focus, textarea:focus {{
            outline: none;
            border-color: #007bff;
        }}
        
        .readonly {{
            background-color: #f8f9fa;
            color: #6c757d;
            cursor: not-allowed;
        }}
        
        .form-section {{
            border: 1px solid #e9ecef;
            border-radius: 6px;
            padding: 20px;
            margin-bottom: 25px;
        }}
        
        .form-section h3 {{
            margin-top: 0;
            margin-bottom: 15px;
            color: #495057;
            font-size: 18px;
        }}
        
        .submit-btn {{
            background-color: #007bff;
            color: white;
            padding: 12px 30px;
            border: none;
            border-radius: 4px;
            font-size: 16px;
            cursor: pointer;
            width: 100%;
            margin-top: 20px;
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
            font-size: 12px;
            color: #6c757d;
            margin-top: 4px;
        }}
        
        .restart-indicator {{
            color: #dc3545;
            font-size: 12px;
            font-weight: 500;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🤖 DisBot Admin Configuration</h1>
        
        {error_html}
        {success_html}
        
        <form method="post" action="/admin">
            <div class="form-section">
                <h3>🔐 Bot Credentials</h3>
                <div class="form-group">
                    <label for="api_key">API Key</label>
                    <input type="text" id="api_key" name="api_key" value="{api_key_display}" readonly class="readonly">
                    <div class="help-text">Bot token - read only for security</div>
                </div>
                
                <div class="form-group">
                    <label for="app_id">Application ID</label>
                    <input type="text" id="app_id" name="app_id" value="{app_id_display}" readonly class="readonly">
                    <div class="help-text">Discord application ID - read only for security</div>
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
                    <textarea id="emote_users" name="emote_users" rows="3" placeholder="User1, User2, User3">{emote_users_display}</textarea>
                    <div class="help-text">Comma-separated list of users for emote reactions</div>
                </div>
                
                <div class="form-group">
                    <label for="env">Environment</label>
                    <select id="env" name="env" required>
                        <option value="prod" {prod_selected}>Production</option>
                        <option value="dev" {dev_selected}>Development</option>
                    </select>
                    <div class="help-text restart-indicator">⚠️ Requires service restart</div>
                </div>
            </div>
            
            <div class="form-section">
                <h3>📊 Logging & Performance</h3>
                <div class="form-group">
                    <label for="log_level">Log Level</label>
                    <select id="log_level" name="log_level" required>
                        <option value="TRACE" {trace_selected}>TRACE</option>
                        <option value="DEBUG" {debug_selected}>DEBUG</option>
                        <option value="INFO" {info_selected}>INFO</option>
                        <option value="WARN" {warn_selected}>WARN</option>
                        <option value="ERROR" {error_selected}>ERROR</option>
                    </select>
                    <div class="help-text restart-indicator">⚠️ Requires service restart</div>
                </div>
                
                <div class="form-group">
                    <label for="voice_channel_timeout_seconds">Voice Channel Timeout (seconds)</label>
                    <input type="number" id="voice_channel_timeout_seconds" name="voice_channel_timeout_seconds" 
                           value="{timeout}" min="10" max="3600" required>
                    <div class="help-text">Time before bot leaves voice channel when inactive (10-3600 seconds)</div>
                </div>
            </div>
            
            <button type="submit" class="submit-btn">💾 Save Configuration</button>
        </form>
    </div>
</body>
</html>"#,
    error_html = error_html,
    success_html = success_html,
    api_key_display = html_escape(api_key_display),
    app_id_display = html_escape(app_id_display),
    emote_name = html_escape(&config.emote_name),
    emote_users_display = html_escape(&emote_users_display),
    prod_selected = if config.env == Environment::Prod {
      "selected"
    } else {
      ""
    },
    dev_selected = if config.env == Environment::Dev {
      "selected"
    } else {
      ""
    },
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
    timeout = config.voice_channel_timeout_seconds,
  )
}

fn html_escape(s: &str) -> String {
  s.replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
    .replace('\'', "&#x27;")
}
