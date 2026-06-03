// src-tauri/src/commands/hackclub.rs
use std::env;
use std::fs;
use std::path::Path;
use serde_json::Value;
use reqwest;

use crate::state::hackclub::HACKCLUB_STATE;
use crate::utils::callback_auth_code_path;

#[tauri::command]
pub async fn start_hackclub_oauth(prompt_login: Option<bool>, max_age: Option<u32>) -> Result<String, String> {
    let client_id = env::var("HACKCLUB_CLIENT_ID")
        .map_err(|_| "Missing HACKCLUB_CLIENT_ID env var".to_string())?;
    println!("DEBUG: start_hackclub_oauth called. HACKCLUB_CLIENT_ID present");
    let redirect_uri = "http://localhost:3001/callback";
    let scope = "openid profile email name slack_id verification_status";

    let mut auth_url = url::Url::parse("https://auth.hackclub.com/oauth/authorize")
        .map_err(|e| format!("Failed to build auth URL: {}", e))?;
    {
        let mut qp = auth_url.query_pairs_mut();
        qp.append_pair("client_id", &client_id)
            .append_pair("scope", scope)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("state", "hackclub");

        if let Some(true) = prompt_login {
            qp.append_pair("prompt", "login");
        }
        if let Some(age) = max_age {
            qp.append_pair("max_age", &age.to_string());
        }
    }

    let url_str = auth_url.to_string();
    println!("DEBUG: auth_url = {}", url_str);
    Ok(url_str)
}

#[tauri::command]
pub async fn get_hackclub_auth_result() -> Result<Value, String> {
    let cached_result = {
        let state = HACKCLUB_STATE.lock().unwrap();
        state.auth_result.clone()
    };

    if let Some(result) = cached_result {
        return Ok(result);
    }

    let auth_code_path = callback_auth_code_path();
    let mut used_path: Option<std::path::PathBuf> = None;

    if auth_code_path.exists() {
        used_path = Some(auth_code_path.clone());
    } else {
        let fallback = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri must have a parent directory")
            .join("callback")
            .join("hackclub_auth_code.txt");
        if fallback.exists() {
            used_path = Some(fallback);
        }
    }

    if let Some(auth_code_path) = used_path {
        let code = fs::read_to_string(&auth_code_path)
            .map_err(|e| format!("Failed to read auth code: {}", e))?
            .trim()
            .to_string();

        if code.is_empty() {
            return Err("No auth result yet".to_string());
        }

        let result = exchange_hackclub_oauth_code(code).await?;
        let _ = fs::remove_file(&auth_code_path);
        Ok(result)
    } else {
        Err("No auth result yet".to_string())
    }
}

#[tauri::command]
pub async fn exchange_hackclub_oauth_code(code: String) -> Result<Value, String> {
    let client_id = env::var("HACKCLUB_CLIENT_ID")
        .map_err(|_| "Missing HACKCLUB_CLIENT_ID env var".to_string())?;
    let client_secret = env::var("HACKCLUB_CLIENT_SECRET")
        .map_err(|_| "Missing HACKCLUB_CLIENT_SECRET env var".to_string())?;
    let redirect_uri = "http://localhost:3001/callback";

    let params = [
        ("client_id".to_string(), client_id),
        ("client_secret".to_string(), client_secret),
        ("code".to_string(), code),
        ("redirect_uri".to_string(), redirect_uri.to_string()),
        ("grant_type".to_string(), "authorization_code".to_string()),
    ];

    let client = reqwest::Client::new();
    let response = client
        .post("https://auth.hackclub.com/oauth/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to exchange code: {}", e))?;
    
    let text = response.text().await.map_err(|e| format!("Failed to read response text: {}", e))?;
    println!("DEBUG: token exchange response: {}", text);
    let mut json = serde_json::from_str::<Value>(&text).map_err(|e| format!("Failed to parse response: {}", e))?;
    
    // Call /api/v1/me to inspect stuff
    if let Some(access_token) = json.get("access_token").and_then(|v| v.as_str()) {
        match client
            .get("https://auth.hackclub.com/api/v1/me")
            .bearer_auth(access_token)
            .send()
            .await
        {
            Ok(me_response) => {
                if let Ok(me_text) = me_response.text().await {
                    println!("DEBUG: /api/v1/me response: {}", me_text);
                    if let Ok(me_json) = serde_json::from_str::<Value>(&me_text) {
                        json["identity_info"] = me_json;
                    }
                }
            }
            Err(e) => {
                println!("DEBUG: Failed to call /api/v1/me: {}", e);
            }
        }
    }
    
    {
        let mut state = HACKCLUB_STATE.lock().unwrap();
        state.auth_result = Some(json.clone());
    }
    
    Ok(json)
}