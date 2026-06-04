use tauri::Emitter;
use crate::state::party::PARTY_STATE;
use crate::commands::wakatime::get_wakatime_today;

#[tauri::command]
pub async fn check_party_time(app_handle: tauri::AppHandle) -> Result<bool, String> {
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
        println!(" PARTY TIME! Crossed {} ten-minute intervals!", current_ten_min_intervals);
        
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