// src-tauri/src/main.rs
mod bms_scan;

use sha2::{Digest, Sha256};
#[allow(unused_imports)]
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
#[allow(unused_imports)]
use tauri::{
    async_runtime::Mutex,
    plugin::{Builder, TauriPlugin},
    Emitter, Manager, Runtime, State, Window,
};
use tokio::fs;
use walkdir::WalkDir;

// 应用状态结构体
#[derive(Default)]
struct AppState {
    // 存储目录与曲目的映射关系
    directories: Mutex<HashMap<String, Vec<Track>>>,
    // 当前导入进度
    current_progress: Mutex<f64>,
}

// 曲目数据结构
#[derive(Clone, serde::Serialize)]
struct Track {
    id: u32,
    title: String,
    artist: String,
    genre: String,
    sha256: String,
}

#[tauri::command]
async fn get_directories(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let dirs = state.directories.lock().await;
    Ok(dirs.keys().cloned().collect())
}

#[tauri::command]
async fn get_tracks(directory: String, state: State<'_, AppState>) -> Result<Vec<Track>, String> {
    let dirs = state.directories.lock().await;
    dirs.get(&directory)
        .cloned()
        .ok_or_else(|| "Directory not found".into())
}

#[tauri::command]
async fn handle_dropped_files(
    window: Window,
    files: Vec<PathBuf>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    for path in files {
        if path.is_dir() {
            process_directory(&window, &state, path).await?;
        } else if let Some(ext) = path.extension() {
            if ext == "zip" {
                // TODO:
                // process_archive(&window, &state, path).await?;
            }
        }
    }
    Ok(())
}

// 添加目录主逻辑
#[tauri::command]
async fn add_directory(
    window: Window,
    path: PathBuf,
    state: State<'_, AppState>,
) -> Result<(), String> {
    process_directory(&window, &state, path).await
}

// 处理目录的异步函数
async fn process_directory(
    window: &Window,
    state: &State<'_, AppState>,
    path: PathBuf,
) -> Result<(), String> {
    let mut tracks = Vec::new();
    let total_files = count_audio_files(&path).await?;
    let mut processed = 0;

    for entry in WalkDir::new(&path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(is_audio_file)
    {
        let track = process_single_file(entry.path()).await?;
        tracks.push(track);

        // 更新进度
        processed += 1;
        let progress = (processed as f64 / total_files as f64) * 100.0;
        *state.current_progress.lock().await = progress;
        window
            .emit("import_progress", progress)
            .map_err(|e| e.to_string())?;
    }

    // 更新状态
    let dir_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid directory name")?
        .to_string();

    let mut dirs = state.directories.lock().await;
    dirs.insert(dir_name, tracks);

    Ok(())
}

// 处理单个音频文件
async fn process_single_file(path: &Path) -> Result<Track, String> {
    // 读取文件内容计算SHA256
    let content = fs::read(path).await.map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let hash = format!("{:x}", hasher.finalize());

    // 解析元数据（示例使用占位值，实际应使用音频文件元数据解析库）
    Ok(Track {
        id: rand::random(),
        title: path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string(),
        artist: "Unknown Artist".into(),
        genre: "Unknown Genre".into(),
        sha256: hash,
    })
}

// 辅助函数：统计音频文件数量
async fn count_audio_files(path: &Path) -> Result<usize, String> {
    let count = WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(is_audio_file)
        .count();
    Ok(count)
}

// 辅助函数：判断是否是音频文件
fn is_audio_file(entry: &walkdir::DirEntry) -> bool {
    entry
        .path()
        .extension()
        .map(|ext| {
            let ext = ext.to_str().unwrap_or("").to_lowercase();
            matches!(ext.as_str(), "mp3" | "wav" | "flac" | "ogg")
        })
        .unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_directories,
            get_tracks,
            handle_dropped_files,
            add_directory
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
