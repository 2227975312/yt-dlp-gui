//! Cookie profile 的 SQLite 持久化
//!
//! 数据库是 cookie 配置的唯一事实源；磁盘上的 `<id>.txt` 只是喂给 yt-dlp 的派生文件。

use super::DatabaseState;
use rusqlite::{params, OptionalExtension};

/// Cookie profile 列表项（不含 cookie 正文，避免无谓地把敏感内容传到前端）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieProfileSummary {
    pub id: String,
    pub name: String,
    pub domains: Vec<String>,
    pub enabled: bool,
    pub sort_order: i64,
    pub cookie_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Cookie profile 完整记录
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieProfile {
    pub id: String,
    pub name: String,
    pub content: String,
    pub domains: Vec<String>,
    pub enabled: bool,
    pub sort_order: i64,
    pub cookie_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 前端提交的 profile 内容
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieProfileInput {
    /// 为空表示新建
    pub id: Option<String>,
    pub name: String,
    /// 为空表示不更新 cookie 正文（沿用库中已有内容）
    #[serde(default)]
    pub content: Option<String>,
    /// 用户确认过的匹配域名；为空时使用按 cookie 正文派生的结果
    pub domains: Vec<String>,
    pub enabled: bool,
}

impl CookieProfileInput {
    /// 校验 profile ID（同时用于拼磁盘文件名，必须挡住路径穿越）
    pub fn normalized_id(&self) -> Result<String, String> {
        match self.id.as_deref() {
            None => Ok(String::new()),
            Some(id) => {
                let valid = !id.is_empty()
                    && id.len() <= 64
                    && id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
                if valid {
                    Ok(id.to_string())
                } else {
                    Err("err_invalid_cookie_profile_id".into())
                }
            }
        }
    }
}

fn parse_domains(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<CookieProfileSummary> {
    let domains: String = row.get(2)?;
    Ok(CookieProfileSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        domains: parse_domains(&domains),
        enabled: row.get::<_, i64>(3)? != 0,
        sort_order: row.get(4)?,
        cookie_count: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

const SUMMARY_COLUMNS: &str =
    "id, name, domains_json, enabled, sort_order, cookie_count, created_at, updated_at";

/// 按用户排序返回全部 profile（不含正文）
pub fn list_profiles(db: &DatabaseState) -> Result<Vec<CookieProfileSummary>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM cookie_profiles ORDER BY sort_order ASC, created_at ASC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_summary)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 读取单个 profile
pub fn get_profile(db: &DatabaseState, id: &str) -> Result<Option<CookieProfile>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        &format!("SELECT {SUMMARY_COLUMNS}, content FROM cookie_profiles WHERE id = ?1"),
        params![id],
        |row| {
            let summary = row_to_summary(row)?;
            Ok(CookieProfile {
                id: summary.id,
                name: summary.name,
                content: row.get(8)?,
                domains: summary.domains,
                enabled: summary.enabled,
                sort_order: summary.sort_order,
                cookie_count: summary.cookie_count,
                created_at: summary.created_at,
                updated_at: summary.updated_at,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// 写入 cookie profile 所需的内容（数据库唯一写入入口的参数束）
pub struct CookieProfileWrite<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub content: &'a str,
    pub domains: &'a [String],
    pub enabled: bool,
    pub cookie_count: usize,
    pub now: i64,
}

/// 写入（新建或更新）profile，返回落库后的完整记录
pub fn upsert_profile(
    db: &DatabaseState,
    write: CookieProfileWrite<'_>,
) -> Result<CookieProfile, String> {
    let CookieProfileWrite {
        id,
        name,
        content,
        domains,
        enabled,
        cookie_count,
        now,
    } = write;
    let domains_json = serde_json::to_string(domains).map_err(|e| e.to_string())?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let existing_sort_order: Option<i64> = conn
        .query_row(
            "SELECT sort_order FROM cookie_profiles WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let sort_order = existing_sort_order.unwrap_or_else(|| {
        conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM cookie_profiles",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default()
    });
    let created_at: Option<i64> = conn
        .query_row(
            "SELECT created_at FROM cookie_profiles WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    conn.execute(
        r#"
        INSERT INTO cookie_profiles (
            id, name, content, domains_json, enabled, sort_order, cookie_count, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            content = excluded.content,
            domains_json = excluded.domains_json,
            enabled = excluded.enabled,
            cookie_count = excluded.cookie_count,
            updated_at = excluded.updated_at
        "#,
        params![
            id,
            name,
            content,
            domains_json,
            if enabled { 1 } else { 0 },
            sort_order,
            cookie_count as i64,
            created_at.unwrap_or(now),
        ],
    )
    .map_err(|e| e.to_string())?;

    // 复用同一把锁读回落库结果（不能再次 lock，std Mutex 不可重入）
    conn.query_row(
        &format!("SELECT {SUMMARY_COLUMNS}, content FROM cookie_profiles WHERE id = ?1"),
        params![id],
        |row| {
            let summary = row_to_summary(row)?;
            Ok(CookieProfile {
                id: summary.id,
                name: summary.name,
                content: row.get(8)?,
                domains: summary.domains,
                enabled: summary.enabled,
                sort_order: summary.sort_order,
                cookie_count: summary.cookie_count,
                created_at: summary.created_at,
                updated_at: summary.updated_at,
            })
        },
    )
    .map_err(|e| e.to_string())
}

/// 删除 profile
pub fn delete_profile(db: &DatabaseState, id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM cookie_profiles WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 按给定顺序重排 profile
pub fn reorder_profiles(db: &DatabaseState, ids: &[String]) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE cookie_profiles SET sort_order = ?1 WHERE id = ?2",
            params![index as i64, id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// 所有启用 profile 的匹配所需字段
pub fn enabled_match_rows(db: &DatabaseState) -> Result<Vec<(String, Vec<String>, i64)>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, domains_json, sort_order FROM cookie_profiles WHERE enabled = 1 ORDER BY sort_order ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let domains: String = row.get(1)?;
            Ok((
                row.get::<_, String>(0)?,
                parse_domains(&domains),
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}
