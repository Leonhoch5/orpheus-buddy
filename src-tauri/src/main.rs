use std::{fs, path::Path};
use git2::Repository;
use std::process::Command;

use rdev::{listen, Event, EventType};
use tauri::Emitter;
use reqwest;
use std::sync::{Arc, Mutex};
use serde_json::Value;
use open;

#[derive(Clone)]
struct SlackOAuthState {
    auth_result: Option<Value>,
}

lazy_static::lazy_static! {
    static ref SLACK_STATE: Arc<Mutex<SlackOAuthState>> = Arc::new(Mutex::new(SlackOAuthState {
        auth_result: None,
    }));
}

#[derive(Clone, Debug)]
struct PartyState {
    last_known_seconds: u32,
    last_party_threshold: u32,
}

lazy_static::lazy_static! {
    static ref PARTY_STATE: Arc<Mutex<PartyState>> = Arc::new(Mutex::new(PartyState {
        last_known_seconds: 0,
        last_party_threshold: 0,
    }));
}

const REPO_URL: &str = "https://github.com/hackclub/dinosaurs.git";
const LOCAL_REPO_DIR: &str = "../public/dinosaurs";
const RESIZED_DIR: &str = "../public/dinosaurs/resized";
const TARGET_SIZE: u32 = 256;

#[tauri::command]
fn update_dinosaurs() -> Result<String, String> {
    let repo_path = Path::new(LOCAL_REPO_DIR);

    let repo = if repo_path.exists() {
        Repository::open(repo_path).map_err(|e| format!("Failed to open repo: {}", e))?
    } else {
        Repository::clone(REPO_URL, repo_path).map_err(|e| format!("Failed to clone repo: {}", e))?
    };

    if repo_path.exists() {
        let mut remote = repo.find_remote("origin").map_err(|e| format!("Failed to find remote: {}", e))?;
        remote.fetch(&["main"], None, None).map_err(|e| format!("Failed to fetch: {}", e))?;

        let fetch_head = repo.find_reference("FETCH_HEAD").map_err(|e| format!("Failed to find FETCH_HEAD: {}", e))?;
        let fetch_commit = fetch_head.peel_to_commit().map_err(|e| format!("Failed to peel to commit: {}", e))?;
        let mut reference = repo.find_reference("refs/heads/main").map_err(|e| format!("Failed to find main branch: {}", e))?;
        reference.set_target(fetch_commit.id(), "Fast-forward").map_err(|e| format!("Failed to fast-forward: {}", e))?;
        repo.set_head("refs/heads/main").map_err(|e| format!("Failed to set HEAD: {}", e))?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force())).map_err(|e| format!("Failed to checkout HEAD: {}", e))?;
    }

    let resized_path = Path::new(RESIZED_DIR);
    if resized_path.exists() {
        fs::remove_dir_all(resized_path).map_err(|e| format!("Failed to remove old resized dir: {}", e))?;
    }
    fs::create_dir_all(resized_path).map_err(|e| format!("Failed to create resized dir: {}", e))?;

    let entries = fs::read_dir(repo_path).map_err(|e| format!("Failed to read repo dir: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e: std::io::Error| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "png") {
            let img = image::open(&path).map_err(|e| format!("Failed to open image {:?}: {}", path, e))?;
            let resized = img.resize_exact(TARGET_SIZE, TARGET_SIZE, image::imageops::FilterType::Lanczos3);
            let filename = path.file_name().unwrap();
            resized.save(resized_path.join(filename)).map_err(|e| format!("Failed to save resized image: {}", e))?;
        }
    }

    Ok(format!("Successfully updated and resized images to {}", TARGET_SIZE))
}

#[tauri::command]
fn clean_dinosaurs() -> Result<String, String> {
    let repo_path = Path::new(LOCAL_REPO_DIR);
    let mut deleted_count = 0;

    let entries = fs::read_dir(repo_path).map_err(|e| format!("Failed to read repo dir: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            fs::remove_file(&path).map_err(|e| format!("Failed to delete {:?}: {}", path, e))?;
            deleted_count += 1;
        }
    }

    Ok(format!("Deleted {} files in dinosaurs folder (folders kept)", deleted_count))
}

#[tauri::command]
fn get_resized_dinosaurs() -> Result<Vec<String>, String> {
    let resized_path = std::path::Path::new("../public/dinosaurs/resized");
    let mut files = Vec::new();
    let entries = std::fs::read_dir(resized_path).map_err(|e| format!("Failed to read resized dir: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "png") {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                files.push(name.to_string());
            }
        }
    }
    Ok(files)
}

#[tauri::command]
async fn get_wakatime_today() -> Result<String, String> {
    println!("=== DEBUG: Getting WakaTime Today Stats ===");
    
    // Try to find wakatime-cli in common locations
    let possible_paths = vec![
        "~/.wakatime/wakatime-cli",
        "wakatime-cli",
        "C:\\Users\\%USERNAME%\\.wakatime\\wakatime-cli.exe",
        "%USERPROFILE%\\.wakatime\\wakatime-cli.exe",
    ];
    
    for path in &possible_paths {
        let expanded_path = if path.contains('~') {
            if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
                path.replace("~", &home)
            } else {
                path.to_string()
            }
        } else if path.contains("%USERPROFILE%") {
            if let Ok(home) = std::env::var("USERPROFILE") {
                path.replace("%USERPROFILE%", &home)
            } else {
                path.to_string()
            }
        } else if path.contains("%USERNAME%") {
            if let Ok(username) = std::env::var("USERNAME") {
                path.replace("%USERNAME%", &username)
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        };
        
        println!("Trying WakaTime CLI path: {}", expanded_path);
        
        let output = Command::new(&expanded_path)
            .arg("--today")
            .arg("--output")
            .arg("json")
            .output();
            
        match output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    println!("✅ WakaTime CLI success: {}", stdout);
                    return Ok(stdout.to_string());
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!("❌ WakaTime CLI failed: {}", stderr);
                }
            }
            Err(e) => {
                println!("❌ Failed to execute {}: {}", expanded_path, e);
            }
        }
    }
    
    Err("WakaTime CLI not found in any common locations".to_string())
}

#[tauri::command]
async fn get_wakatime_today_detailed() -> Result<String, String> {
    println!("=== DEBUG: Getting Detailed WakaTime Today Stats ===");
    
    let possible_paths = vec![
        "~/.wakatime/wakatime-cli",
        "wakatime-cli", 
        "C:\\Users\\%USERNAME%\\.wakatime\\wakatime-cli.exe",
        "%USERPROFILE%\\.wakatime\\wakatime-cli.exe",
    ];
    
    for path in &possible_paths {
        let expanded_path = if path.contains('~') {
            if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
                path.replace("~", &home)
            } else {
                path.to_string()
            }
        } else if path.contains("%USERPROFILE%") {
            if let Ok(home) = std::env::var("USERPROFILE") {
                path.replace("%USERPROFILE%", &home)
            } else {
                path.to_string()
            }
        } else if path.contains("%USERNAME%") {
            if let Ok(username) = std::env::var("USERNAME") {
                path.replace("%USERNAME%", &username)
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        };
        
        println!("Trying WakaTime CLI path for detailed stats: {}", expanded_path);
        
        // Use --today flag with raw-json output for detailed breakdown
        let output = Command::new(&expanded_path)
            .arg("--today")
            .arg("--output")
            .arg("raw-json")
            .output();
            
        match output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    println!("✅ WakaTime CLI detailed success: {}", stdout);
                    return Ok(stdout.to_string());
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!("❌ WakaTime CLI detailed failed: {}", stderr);
                    
                    let fallback_output = Command::new(&expanded_path)
                        .arg("--today")
                        .arg("--output")
                        .arg("json")
                        .output();
                        
                    match fallback_output {
                        Ok(fallback_result) => {
                            if fallback_result.status.success() {
                                let fallback_stdout = String::from_utf8_lossy(&fallback_result.stdout);
                                println!("✅ WakaTime CLI fallback success: {}", fallback_stdout);
                                return Ok(format!("{{\"fallback\":true,\"data\":{}}}", fallback_stdout));
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to execute detailed stats {}: {}", expanded_path, e);
            }
        }
    }
    
    Err("WakaTime CLI not found in any common locations".to_string())
}

#[tauri::command]
async fn get_wakatime_stats() -> Result<String, String> {
    println!("=== DEBUG: Getting WakaTime Stats ===");
    
    // Try to find wakatime-cli in common locations
    let possible_paths = vec![
        "~/.wakatime/wakatime-cli",
        "wakatime-cli",
        "C:\\Users\\%USERNAME%\\.wakatime\\wakatime-cli.exe",
        "%USERPROFILE%\\.wakatime\\wakatime-cli.exe",
    ];
    
    for path in &possible_paths {
        let expanded_path = if path.contains('~') {
            if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
                path.replace("~", &home)
            } else {
                path.to_string()
            }
        } else if path.contains("%USERPROFILE%") {
            if let Ok(home) = std::env::var("USERPROFILE") {
                path.replace("%USERPROFILE%", &home)
            } else {
                path.to_string()
            }
        } else if path.contains("%USERNAME%") {
            if let Ok(username) = std::env::var("USERNAME") {
                path.replace("%USERNAME%", &username)
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        };
        
        println!("Trying WakaTime CLI path for stats: {}", expanded_path);
        
        let version_check = Command::new(&expanded_path)
            .arg("--version")
            .output();
            
        if version_check.is_err() {
            println!("❌ CLI not found at: {}", expanded_path);
            continue;
        }
        
        let output = Command::new(&expanded_path)
            .arg("--offline-count")
            .arg("--output")
            .arg("json")
            .output();
            
        match output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    println!("✅ WakaTime CLI offline count success: {}", stdout);
                    return Ok(format!("{{\"type\":\"offline_count\",\"data\":{}}}", stdout.trim()));
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!("❌ WakaTime CLI offline count failed: {}", stderr);
                }
            }
            Err(e) => {
                println!("❌ Failed to execute offline count: {}", e);
            }
        }
        
        let config_output = Command::new(&expanded_path)
            .arg("--config-read")
            .arg("api_key")
            .output();
            
        match config_output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    let api_key_preview = if stdout.len() > 10 {
                        format!("{}...", &stdout[..10])
                    } else {
                        stdout.to_string()
                    };
                    println!("✅ WakaTime API key found: {}", api_key_preview);
                    return Ok(format!("{{\"type\":\"config\",\"api_key_preview\":\"{}\",\"status\":\"configured\"}}", api_key_preview.trim()));
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!("❌ WakaTime config read failed: {}", stderr);
                    return Ok("{\"type\":\"config\",\"status\":\"not_configured\"}".to_string());
                }
            }
            Err(e) => {
                println!("❌ Failed to read config: {}", e);
            }
        }
    }
    
    Err("WakaTime CLI not found in any common locations".to_string())
}

#[tauri::command]
async fn start_slack_oauth() -> Result<String, String> {
    let client_id = "2210535565.9333084041156";
    let redirect_uri = "https://127.0.0.1:8080/callback";
    let scope = "chat:write,channels:read,users:read";
    
    let auth_url = format!(
        "https://slack.com/oauth/v2/authorize?client_id={}&scope={}&redirect_uri={}",
        client_id, scope, redirect_uri
    );
    
    Ok(auth_url)
}

#[tauri::command]
async fn get_slack_auth_result() -> Result<Value, String> {
    let state = SLACK_STATE.lock().unwrap();
    if let Some(ref result) = state.auth_result {
        Ok(result.clone())
    } else {
        Err("No auth result yet".to_string())
    }
}

#[tauri::command]
async fn exchange_slack_oauth_code(code: String) -> Result<Value, String> {
    let client_id = "2210535565.9333084041156";
    let client_secret = "a99334e63ae05a24fba0127b4a0db577";
    let redirect_uri = "https://127.0.0.1:8080/callback";
    
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code", &code),
        ("redirect_uri", redirect_uri),
    ];
    
    let client = reqwest::Client::new();
    let response = client
        .post("https://slack.com/api/oauth.v2.access")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to exchange code: {}", e))?;
    
    let json = response.json::<Value>().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    {
        let mut state = SLACK_STATE.lock().unwrap();
        state.auth_result = Some(json.clone());
    }
    
    Ok(json)
}

async fn exchange_slack_code(code: String) {
    let client_id = "2210535565.9333084041156";
    let client_secret = "a99334e63ae05a24fba0127b4a0db577";
    let redirect_uri = "https://tauri.localhost/slack/callback";
    
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code", &code),
        ("redirect_uri", redirect_uri),
    ];
    
    let client = reqwest::Client::new();
    match client
        .post("https://slack.com/api/oauth.v2.access")
        .form(&params)
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(json) = response.json::<Value>().await {
                let mut state = SLACK_STATE.lock().unwrap();
                state.auth_result = Some(json);
            }
        }
        Err(e) => {
            eprintln!("Failed to exchange code: {}", e);
        }
    }
}
#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    open::that(url).map_err(|e| format!("Failed to open URL: {}", e))
}
fn start_keyboard_listener(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        listen(move |event: Event| {
            if let EventType::KeyPress(_) = event.event_type {
                app_handle.emit("global_keypress", {}).unwrap();
            }
        }).unwrap();
    });
}

#[tauri::command]
async fn check_party_time(app_handle: tauri::AppHandle) -> Result<bool, String> {
    
    let today_result = get_wakatime_today().await?;
    
    let parsed: serde_json::Value = serde_json::from_str(&today_result)
        .map_err(|e| format!("Failed to parse WakaTime data: {}", e))?;
    
    let current_seconds = if let Some(total_seconds) = parsed.get("total_seconds") {
        total_seconds.as_u64().unwrap_or(0) as u32
    } else if let Some(data) = parsed.get("data") {
        if let Some(grand_total) = data.get("grand_total") {
            grand_total.get("total_seconds").and_then(|s| s.as_f64()).unwrap_or(0.0) as u32
        } else {
            0
        }
    } else {
        if let Some(text) = parsed.get("text") {
            let text_str = text.as_str().unwrap_or("");
            parse_time_text_to_seconds(text_str)
        } else {
            0
        }
    };
    
    println!("Current coding time: {} seconds ({} minutes)", current_seconds, current_seconds / 60);
    
    let mut party_state = PARTY_STATE.lock().unwrap();
    
    let current_ten_min_intervals = current_seconds / 600; 
    let last_ten_min_intervals = party_state.last_party_threshold / 600;
    
    println!("Current 10-min intervals: {}, Last intervals: {}", current_ten_min_intervals, last_ten_min_intervals);
    
    if current_ten_min_intervals > last_ten_min_intervals && current_seconds > party_state.last_known_seconds {
        println!("🎉 PARTY TIME! Crossed {} ten-minute intervals!", current_ten_min_intervals);
        
        party_state.last_known_seconds = current_seconds;
        party_state.last_party_threshold = current_ten_min_intervals * 600;
        
        app_handle.emit(
            "party_time",
            serde_json::json!({
                "minutes_coded": current_seconds / 60,
                "intervals_completed": current_ten_min_intervals
            })
        ).map_err(|e| format!("Failed to emit party event: {}", e))?;
        
        return Ok(true);
    } else {
        party_state.last_known_seconds = current_seconds;
        println!("No party yet - need {} more seconds for next party", 
                 ((current_ten_min_intervals + 1) * 600) - current_seconds);
    }
    
    Ok(false)
}

fn parse_time_text_to_seconds(text: &str) -> u32 {
    let mut total_seconds = 0u32;
    
    if let Some(h_pos) = text.find('h') {
        if let Ok(hours) = text[..h_pos].trim().parse::<u32>() {
            total_seconds += hours * 3600;
        }
    }
    
    if let Some(m_pos) = text.find('m') {
        let start = if text.contains('h') {
            text.find('h').unwrap() + 1
        } else {
            0
        };
        if let Ok(minutes) = text[start..m_pos].trim().parse::<u32>() {
            total_seconds += minutes * 60;
        }
    }
    
    if let Some(s_pos) = text.find('s') {
        let start = if text.contains('m') {
            text.find('m').unwrap() + 1
        } else if text.contains('h') {
            text.find('h').unwrap() + 1
        } else {
            0
        };
        if let Ok(seconds) = text[start..s_pos].trim().parse::<u32>() {
            total_seconds += seconds;
        }
    }
    
    total_seconds
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            start_keyboard_listener(app_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            update_dinosaurs,
            clean_dinosaurs,
            get_resized_dinosaurs,
            get_wakatime_today,
            get_wakatime_today_detailed,
            get_wakatime_stats,
            start_slack_oauth,
            get_slack_auth_result,
            exchange_slack_oauth_code,
            open_url,
            check_party_time
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
