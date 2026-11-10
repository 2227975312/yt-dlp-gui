//! 视频信息获取

use serde_json::Value;
use tauri::AppHandle;

use super::support;

// 视频信息

/// 使用 yt-dlp -J 获取视频元信息（标题、格式列表、字幕等）
#[tauri::command]
pub async fn fetch_video_info(
    app: AppHandle,
    url: String,
    cookie_file: Option<String>,
    proxy: Option<String>,
) -> Result<Value, String> {
    support::run_ytdlp_json(&app, &url, &[], cookie_file.as_deref(), proxy.as_deref()).await
}
