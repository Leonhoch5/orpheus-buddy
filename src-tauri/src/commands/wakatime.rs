use std::process::Command;

#[tauri::command]
pub async fn get_wakatime_today() -> Result<String, String> {
    println!("=== DEBUG: Getting WakaTime Today Stats ===");
    
    let possible_paths = vec![
        "~/.wakatime/wakatime-cli",
        "wakatime-cli", 
        "C:\\Users\\%USERNAME%\\.wakatime\\wakatime-cli.exe",
        "%USERPROFILE%\\.wakatime\\wakatime-cli.exe",
    ];
    
    for path in &possible_paths {
        let expanded_path = expand_path(path);
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
                    println!("WakaTime CLI success: {}", stdout);
                    return Ok(stdout.to_string());
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!("WakaTime CLI failed: {}", stderr);
                }
            }
            Err(e) => {
                println!("Failed to execute {}: {}", expanded_path, e);
            }
        }
    }
    
    Err("WakaTime CLI not found in any common locations".to_string())
}

#[tauri::command]
pub async fn get_wakatime_today_detailed() -> Result<String, String> {
    println!("=== DEBUG: Getting Detailed WakaTime Today Stats ===");
    
    let possible_paths = vec![
        "~/.wakatime/wakatime-cli",
        "wakatime-cli", 
        "C:\\Users\\%USERNAME%\\.wakatime\\wakatime-cli.exe",
        "%USERPROFILE%\\.wakatime\\wakatime-cli.exe",
    ];
    
    for path in &possible_paths {
        let expanded_path = expand_path(path);
        println!("Trying WakaTime CLI path for detailed stats: {}", expanded_path);
        
        let output = Command::new(&expanded_path)
            .arg("--today")
            .arg("--output")
            .arg("raw-json")
            .output();
            
        match output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    println!("WakaTime CLI detailed success: {}", stdout);
                    return Ok(stdout.to_string());
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!("WakaTime CLI detailed failed: {}", stderr);
                    
                    let fallback_output = Command::new(&expanded_path)
                        .arg("--today")
                        .arg("--output")
                        .arg("json")
                        .output();
                        
                    match fallback_output {
                        Ok(fallback_result) => {
                            if fallback_result.status.success() {
                                let fallback_stdout = String::from_utf8_lossy(&fallback_result.stdout);
                                println!("WakaTime CLI fallback success: {}", fallback_stdout);
                                return Ok(format!("{{\"fallback\":true,\"data\":{}}}", fallback_stdout));
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            Err(e) => {
                println!("Failed to execute detailed stats {}: {}", expanded_path, e);
            }
        }
    }
    
    Err("WakaTime CLI not found in any common locations".to_string())
}

#[tauri::command]
pub async fn get_wakatime_stats() -> Result<String, String> {
    println!("=== DEBUG: Getting WakaTime Stats ===");
    
    let possible_paths = vec![
        "~/.wakatime/wakatime-cli",
        "wakatime-cli",
        "C:\\Users\\%USERNAME%\\.wakatime\\wakatime-cli.exe",
        "%USERPROFILE%\\.wakatime\\wakatime-cli.exe",
    ];
    
    for path in &possible_paths {
        let expanded_path = expand_path(path);
        println!("Trying WakaTime CLI path for stats: {}", expanded_path);
        
        let version_check = Command::new(&expanded_path)
            .arg("--version")
            .output();
            
        if version_check.is_err() {
            println!("CLI not found at: {}", expanded_path);
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
                    println!("WakaTime CLI offline count success: {}", stdout);
                    return Ok(format!("{{\"type\":\"offline_count\",\"data\":{}}}", stdout.trim()));
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!("WakaTime CLI offline count failed: {}", stderr);
                }
            }
            Err(e) => {
                println!("Failed to execute offline count: {}", e);
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
                    println!("WakaTime API key found: {}", api_key_preview);
                    return Ok(format!("{{\"type\":\"config\",\"api_key_preview\":\"{}\",\"status\":\"configured\"}}", api_key_preview.trim()));
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!("WakaTime config read failed: {}", stderr);
                    return Ok("{\"type\":\"config\",\"status\":\"not_configured\"}".to_string());
                }
            }
            Err(e) => {
                println!("Failed to read config: {}", e);
            }
        }
    }
    
    Err("WakaTime CLI not found in any common locations".to_string())
}

fn expand_path(path: &str) -> String {
    if path.contains('~') {
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
    }
}