use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use serde_json::Value;
use reqwest;
use tauri::Emitter;
use lazy_static::lazy_static;

use crate::state::slack::{SLACK_STATE, SLACK_NOTIF_STATE};

#[tauri::command]
pub async fn start_slack_oauth(_reauth: Option<bool>) -> Result<String, String> {
    let client_id = env::var("SLACK_CLIENT_ID")
        .map_err(|_| "Missing SLACK_CLIENT_ID env var".to_string())?;
    let redirect_uri = "http://localhost:3001/callback";
    let scope = "channels:read channels:history im:read im:history users:read";
    
    let mut auth_url = url::Url::parse("https://slack.com/oauth/v2/authorize")
        .map_err(|e| format!("Failed to build Slack auth URL: {}", e))?;
    {
        let mut qp = auth_url.query_pairs_mut();
        qp.append_pair("client_id", &client_id)
            .append_pair("scope", scope)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("state", "slack");
    }

    Ok(auth_url.to_string())
}

#[tauri::command]
pub async fn get_slack_auth_result() -> Result<Value, String> {
    let auth_code_path = slack_auth_code_path();
    let code_file_exists = auth_code_path.exists();
    
    if !code_file_exists {
        let cached_result = {
            let state = SLACK_STATE.lock().unwrap();
            state.auth_result.clone()
        };
        if let Some(result) = cached_result {
            println!("DEBUG: returning cached slack auth result");
            return Ok(result);
        } else {
            return Err("No slack auth result yet".to_string());
        }
    }
    
    println!("DEBUG: found slack auth code file at {:?}", auth_code_path);
    let code = fs::read_to_string(&auth_code_path)
        .map_err(|e| format!("Failed to read slack auth code: {}", e))?
        .trim()
        .to_string();

    if code.is_empty() {
        return Err("No slack auth result yet".to_string());
    }

    let result = exchange_slack_oauth_code(code).await;
    let _ = fs::remove_file(&auth_code_path);
    
    match result {
        Ok(json) => Ok(json),
        Err(e) => {
            {
                let mut state = SLACK_STATE.lock().unwrap();
                state.auth_result = None;
            }
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn exchange_slack_oauth_code(code: String) -> Result<Value, String> {
    let client_id = env::var("SLACK_CLIENT_ID")
        .map_err(|_| "Missing SLACK_CLIENT_ID env var".to_string())?;
    let client_secret = env::var("SLACK_CLIENT_SECRET")
        .map_err(|_| "Missing SLACK_CLIENT_SECRET env var".to_string())?;
    let redirect_uri = "http://localhost:3001/callback";

    let params = [
        ("client_id".to_string(), client_id),
        ("client_secret".to_string(), client_secret),
        ("code".to_string(), code.trim().to_string()),
        ("redirect_uri".to_string(), redirect_uri.to_string()),
    ];

    let client = reqwest::Client::new();
    
    for attempt in 0..3 {
        let response = client
            .post("https://slack.com/api/oauth.v2.access")
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to exchange slack code: {}", e))?;

        let text = response.text().await.map_err(|e| format!("Failed to read response text: {}", e))?;
        println!("DEBUG: slack token exchange response (attempt {}): {}", attempt + 1, text);
        
        let json: Value = serde_json::from_str(&text).map_err(|e| format!("Failed to parse response: {}", e))?;
        
        if let Some(false) = json.get("ok").and_then(|v| v.as_bool()) {
            let error = json.get("error").and_then(|v| v.as_str()).unwrap_or("unknown_error");
            if error == "internal_error" && attempt < 2 {
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                continue;
            }
            return Err(format!("Slack OAuth error: {}", error));
        }
        
        if json.get("access_token").is_some() {
            {
                let mut state = SLACK_STATE.lock().unwrap();
                state.auth_result = Some(json.clone());
            }
            return Ok(json);
        } else {
            return Err("No access_token in response".to_string());
        }
    }
    
    Err("Failed to exchange code after retries".to_string())
}

#[tauri::command]
pub async fn start_slack_notification_poller(app_handle: tauri::AppHandle) -> Result<(), String> {
    let slack_token = get_slack_token().ok_or("Slack not authenticated".to_string())?;
    
    {
        let client = reqwest::Client::new();
        let resp = client
            .post("https://slack.com/api/auth.test")
            .bearer_auth(&slack_token)
            .send()
            .await
            .map_err(|e| format!("Failed to get user ID: {}", e))?;
        
        let json: Value = resp.json().await.map_err(|e| format!("Failed to parse auth.test: {}", e))?;
        if let Some(user_id) = json.get("user_id").and_then(|v| v.as_str()) {
            let mut notif_state = SLACK_NOTIF_STATE.lock().unwrap();
            notif_state.user_id = Some(user_id.to_string());
            println!("DEBUG: Slack user ID = {}", user_id);
        }
    }
    
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                if let Err(e) = poll_slack_notifications(&slack_token, &app_handle).await {
                    eprintln!("DEBUG: Slack poll error: {}", e);
                }
            }
        });
    });
    
    Ok(())
}

async fn poll_slack_notifications(token: &str, app_handle: &tauri::AppHandle) -> Result<(), String> {
    let client = reqwest::Client::new();
    
    let user_id = {
        let state = SLACK_NOTIF_STATE.lock().unwrap();
        state.user_id.clone()
    };
    let user_id = user_id.ok_or("User ID not set".to_string())?;
    
    {
        let last_ts = {
            let state = SLACK_NOTIF_STATE.lock().unwrap();
            state.last_dm_ts.clone()
        };
        
        let resp = client
            .post("https://slack.com/api/conversations.list")
            .bearer_auth(token)
            .form(&[("types", "im"), ("limit", "20")])
            .send()
            .await
            .map_err(|e| format!("Failed to list DMs: {}", e))?;
        
        let json: Value = resp.json().await.map_err(|e| format!("Failed to parse DM list: {}", e))?;
        
        if let Some(channels) = json.get("channels").and_then(|v| v.as_array()) {
            for channel in channels {
                if let Some(channel_id) = channel.get("id").and_then(|v| v.as_str()) {
                    let hist_resp = client
                        .post("https://slack.com/api/conversations.history")
                        .bearer_auth(token)
                        .form(&[
                            ("channel", channel_id),
                            ("oldest", &last_ts),
                            ("limit", "10"),
                        ])
                        .send()
                        .await
                        .map_err(|e| format!("Failed to get DM history: {}", e))?;
                    
                    let hist_json: Value = hist_resp.json().await.map_err(|e| format!("Failed to parse DM history: {}", e))?;
                    
                    if let Some(messages) = hist_json.get("messages").and_then(|v| v.as_array()) {
                        for msg in messages {
                            if let Some(ts) = msg.get("ts").and_then(|v| v.as_str()) {
                                if let Some(text) = msg.get("text").and_then(|v| v.as_str()) {
                                    app_handle.emit("slack_notification", serde_json::json!({
                                        "type": "dm",
                                        "text": text,
                                        "ts": ts
                                    })).ok();
                                    
                                    let mut state = SLACK_NOTIF_STATE.lock().unwrap();
                                    state.last_dm_ts = ts.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    {
        lazy_static! {
            static ref CHANNEL_LAST_TS: std::sync::Mutex<HashMap<String, String>> = std::sync::Mutex::new(HashMap::new());
        }
        
        let channels_resp = client
            .post("https://slack.com/api/conversations.list")
            .bearer_auth(token)
            .form(&[("types", "public_channel"), ("limit", "50")])
            .send()
            .await
            .map_err(|e| format!("Failed to list public channels: {}", e))?;
        
        let channels_json: Value = channels_resp.json().await.map_err(|e| format!("Failed to parse channel list: {}", e))?;
        
        if let Some(channels) = channels_json.get("channels").and_then(|v| v.as_array()) {
            for channel in channels {
                if let Some(channel_id) = channel.get("id").and_then(|v| v.as_str()) {
                    let last_ts = {
                        let map = CHANNEL_LAST_TS.lock().unwrap();
                        map.get(channel_id).cloned().unwrap_or_else(|| "0".to_string())
                    };
                    
                    let hist_resp = client
                        .post("https://slack.com/api/conversations.history")
                        .bearer_auth(token)
                        .form(&[
                            ("channel", channel_id),
                            ("oldest", &last_ts),
                            ("limit", "20"),
                        ])
                        .send()
                        .await
                        .map_err(|e| format!("Failed to get channel history for {}: {}", channel_id, e))?;
                    
                    let hist_json: Value = hist_resp.json().await.map_err(|e| format!("Failed to parse channel history: {}", e))?;
                    
                    if let Some(messages) = hist_json.get("messages").and_then(|v| v.as_array()) {
                        let mut new_last_ts = last_ts.clone();
                        for msg in messages {
                            if let Some(ts) = msg.get("ts").and_then(|v| v.as_str()) {
                                if ts > new_last_ts.as_str() {
                                    new_last_ts = ts.to_string();
                                }
                                if let Some(text) = msg.get("text").and_then(|v| v.as_str()) {
                                    if text.contains(&format!("<@{}>", user_id)) {
                                        app_handle.emit("slack_notification", serde_json::json!({
                                            "type": "mention",
                                            "channel_id": channel_id,
                                            "text": text,
                                            "ts": ts
                                        })).ok();
                                    }
                                }
                            }
                        }
                        {
                            let mut map = CHANNEL_LAST_TS.lock().unwrap();
                            map.insert(channel_id.to_string(), new_last_ts);
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

fn get_slack_token() -> Option<String> {
    let state = SLACK_STATE.lock().unwrap();
    state.auth_result.as_ref()
        .and_then(|v| v.get("access_token"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn slack_auth_code_path() -> std::path::PathBuf {
    if let Some(mut dir) = dirs::data_dir() {
        dir.push("orpheus-buddy");
        let _ = std::fs::create_dir_all(&dir);
        dir.push("slack_auth_code.txt");
        dir
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri must have a parent directory")
            .join("callback")
            .join("slack_auth_code.txt")
    }
}