//! Cookie profile 管理命令
//!
//! 数据库是唯一事实源，`<id>.txt` 是按需重建的派生文件：只在保存与解析命中时写盘。

use crate::cookies::{matcher, netscape};
use crate::db::{cookies as db_cookies, DatabaseState};
use crate::utils;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, State};

use db_cookies::{CookieProfile, CookieProfileInput, CookieProfileSummary, CookieProfileWrite};

/// 新建 profile 时的本地自增计数，避免同一毫秒内产生重复 ID
static PROFILE_ID_SEQ: AtomicU64 = AtomicU64::new(0);

/// 命中结果：既有 profile 元信息，也有可直接传给 yt-dlp 的文件路径
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieMatchResult {
    pub profile_id: String,
    pub profile_name: String,
    /// 命中的规则，用于向用户解释为什么选了这个 profile
    pub matched_domain: String,
    pub cookie_file: String,
}

fn next_profile_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let seq = PROFILE_ID_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("ck_{millis}_{seq}")
}

/// 原子写入 profile 派生文件
async fn write_cookie_file(app: &AppHandle, id: &str, content: &str) -> Result<String, String> {
    let path = utils::get_cookie_profile_path(app, id)?;
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| format!("err_save_cookie:{e}"))?;
    }
    let temporary = path.with_extension("txt.tmp");
    tokio::fs::write(&temporary, content.as_bytes())
        .await
        .map_err(|e| format!("err_save_cookie:{e}"))?;
    // Windows 上 rename 无法覆盖已存在文件，先删目标再重试
    if tokio::fs::rename(&temporary, &path).await.is_err() {
        let _ = tokio::fs::remove_file(&path).await;
        tokio::fs::rename(&temporary, &path)
            .await
            .map_err(|e| format!("err_save_cookie:{e}"))?;
    }
    Ok(path.to_string_lossy().to_string())
}

/// 取 profile 派生文件路径，文件缺失时从数据库重建
async fn ensure_cookie_file(
    app: &AppHandle,
    db: &DatabaseState,
    id: &str,
) -> Result<String, String> {
    let path = utils::get_cookie_profile_path(app, id)?;
    if tokio::fs::metadata(&path).await.is_ok() {
        return Ok(path.to_string_lossy().to_string());
    }
    let profile =
        db_cookies::get_profile(db, id)?.ok_or_else(|| "err_cookie_profile_missing".to_string())?;
    write_cookie_file(app, id, &profile.content).await
}

/// 列出全部 cookie profile（不含正文）
#[tauri::command]
pub fn cookie_profile_list(
    db: State<'_, DatabaseState>,
) -> Result<Vec<CookieProfileSummary>, String> {
    db_cookies::list_profiles(&db)
}

/// 取指定 profile 交给 yt-dlp 的派生文件路径（文件缺失时重建）
#[tauri::command]
pub async fn cookie_profile_file(
    app: AppHandle,
    db: State<'_, DatabaseState>,
    id: String,
) -> Result<String, String> {
    ensure_cookie_file(&app, &db, &id).await
}

/// 新建或更新 cookie profile
///
/// cookie 正文不回显给前端，因此 `content` 为空表示「沿用库中已有正文」；
/// 此时若 profile 不存在（或为新建）则报错，避免产生没有正文的空配置。
#[tauri::command]
pub async fn cookie_profile_save(
    app: AppHandle,
    db: State<'_, DatabaseState>,
    input: CookieProfileInput,
) -> Result<CookieProfile, String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("err_empty_cookie_profile_name".into());
    }
    let requested_id = input.normalized_id()?;
    let id = if requested_id.is_empty() {
        next_profile_id()
    } else {
        requested_id
    };
    let existing = db_cookies::get_profile(&db, &id)?;
    if input.id.is_some() && existing.is_none() {
        return Err("err_cookie_profile_missing".into());
    }

    let submitted = input.content.unwrap_or_default();
    let (content, derived_domains, cookie_count) = if submitted.trim().is_empty() {
        let Some(existing) = &existing else {
            return Err("err_empty_cookie_file".into());
        };
        (
            existing.content.clone(),
            existing.domains.clone(),
            existing.cookie_count as usize,
        )
    } else {
        let parsed = netscape::parse(&submitted)?;
        (parsed.normalized, parsed.domains, parsed.cookie_count)
    };

    // 用户确认过的域名优先；为空时回退到按 cookie 正文派生（或已有域名）
    let domains = if input.domains.iter().any(|rule| !rule.trim().is_empty()) {
        matcher::normalize_domains(&input.domains)
    } else {
        derived_domains
    };
    if domains.is_empty() {
        return Err("err_empty_cookie_domains".into());
    }

    write_cookie_file(&app, &id, &content).await?;
    db_cookies::upsert_profile(
        &db,
        CookieProfileWrite {
            id: &id,
            name,
            content: &content,
            domains: &domains,
            enabled: input.enabled,
            cookie_count,
            now: now_millis(),
        },
    )
}

/// 删除 cookie profile 及其派生文件
#[tauri::command]
pub async fn cookie_profile_delete(
    app: AppHandle,
    db: State<'_, DatabaseState>,
    id: String,
) -> Result<(), String> {
    db_cookies::delete_profile(&db, &id)?;
    if let Ok(path) = utils::get_cookie_profile_path(&app, &id) {
        let _ = tokio::fs::remove_file(path).await;
    }
    Ok(())
}

/// 按给定顺序重排 profile
#[tauri::command]
pub fn cookie_profile_reorder(
    db: State<'_, DatabaseState>,
    ids: Vec<String>,
) -> Result<(), String> {
    db_cookies::reorder_profiles(&db, &ids)
}

/// 校验 cookie 文本并返回可用的匹配域名（编辑时实时预填）
#[tauri::command]
pub fn cookie_preview_domains(text: String) -> Result<Vec<String>, String> {
    Ok(netscape::parse(&text)?.domains)
}

/// 按链接匹配 cookie profile，命中时返回可直接使用的派生文件
#[tauri::command]
pub async fn cookie_profile_match(
    app: AppHandle,
    db: State<'_, DatabaseState>,
    url: String,
) -> Result<Option<CookieMatchResult>, String> {
    let rows = db_cookies::enabled_match_rows(&db)?;
    let candidates: Vec<matcher::MatchCandidate<'_>> = rows
        .iter()
        .map(|(id, domains, sort_order)| matcher::MatchCandidate {
            id,
            domains,
            sort_order: *sort_order,
        })
        .collect();
    let Some(matched) = matcher::match_profile(&candidates, &url) else {
        return Ok(None);
    };
    let profile_id = matched.id.to_string();
    let matched_domain = matched.domain.to_string();
    let profile = db_cookies::get_profile(&db, &profile_id)?
        .ok_or_else(|| "err_cookie_profile_missing".to_string())?;
    let cookie_file = ensure_cookie_file(&app, &db, &profile_id).await?;
    Ok(Some(CookieMatchResult {
        profile_id,
        profile_name: profile.name,
        matched_domain,
        cookie_file,
    }))
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
