use std::{fs, path::Path};
use git2::Repository;
use image::GenericImageView;
use tauri::Manager;
use rdev::{listen, Event, EventType};
use tauri::Emitter; // <-- Add this
use reqwest; // <-- Add this

const REPO_URL: &str = "https://github.com/hackclub/dinosaurs.git";
const LOCAL_REPO_DIR: &str = "dinosaurs"; // relative to app executable path
const RESIZED_DIR: &str = "dinosaurs/resized";
const TARGET_SIZE: u32 = 256;

#[tauri::command]
fn update_dinosaurs() -> Result<String, String> {
    let repo_path = Path::new(LOCAL_REPO_DIR);

    // Clone or open repo
    let repo = if repo_path.exists() {
        Repository::open(repo_path).map_err(|e| format!("Failed to open repo: {}", e))?
    } else {
        Repository::clone(REPO_URL, repo_path).map_err(|e| format!("Failed to clone repo: {}", e))?
    };

    // Pull latest changes if already cloned
    if repo_path.exists() {
        let mut remote = repo.find_remote("origin").map_err(|e| format!("Failed to find remote: {}", e))?;
        remote.fetch(&["main"], None, None).map_err(|e| format!("Failed to fetch: {}", e))?;

        // Reset to latest remote/main
        let fetch_head = repo.find_reference("FETCH_HEAD").map_err(|e| format!("Failed to find FETCH_HEAD: {}", e))?;
        let fetch_commit = fetch_head.peel_to_commit().map_err(|e| format!("Failed to peel to commit: {}", e))?;
        let mut reference = repo.find_reference("refs/heads/main").map_err(|e| format!("Failed to find main branch: {}", e))?;
        reference.set_target(fetch_commit.id(), "Fast-forward").map_err(|e| format!("Failed to fast-forward: {}", e))?;
        repo.set_head("refs/heads/main").map_err(|e| format!("Failed to set HEAD: {}", e))?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force())).map_err(|e| format!("Failed to checkout HEAD: {}", e))?;
    }

    // Prepare resized directory
    let resized_path = Path::new(RESIZED_DIR);
    if resized_path.exists() {
        fs::remove_dir_all(resized_path).map_err(|e| format!("Failed to remove old resized dir: {}", e))?;
    }
    fs::create_dir_all(resized_path).map_err(|e| format!("Failed to create resized dir: {}", e))?;

    // Resize PNG images
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

        // Delete if it's a file (not a folder)
        if path.is_file() {
            fs::remove_file(&path).map_err(|e| format!("Failed to delete {:?}: {}", path, e))?;
            deleted_count += 1;
        }
    }

    Ok(format!("Deleted {} files in dinosaurs folder (folders kept)", deleted_count))
}

#[tauri::command]
fn get_resized_dinosaurs() -> Result<Vec<String>, String> {
    let resized_path = std::path::Path::new("dinosaurs/resized");
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
async fn fetch_hackatime_stats(api_url: String, api_key: String) -> Result<String, String> {
    println!("Received API URL: {}", api_url);
    println!("Received API Key: {}", api_key);

    let client = reqwest::Client::new();
    let res = client
        .get(&api_url)
        .bearer_auth(&api_key)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
        
    let text = res.text().await.map_err(|e| format!("Read failed: {}", e))?;
    Ok(text)
}

fn start_keyboard_listener(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        listen(move |event: Event| {
            if let EventType::KeyPress(_) = event.event_type {
                // Send a Tauri event to frontend
                app_handle.emit("global_keypress", {}).unwrap();
            }
        }).unwrap();
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            update_dinosaurs,
            clean_dinosaurs,
            get_resized_dinosaurs,
            fetch_hackatime_stats
        ])
        .setup(|app| {
            start_keyboard_listener(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
