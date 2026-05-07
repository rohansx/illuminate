// Format knowledge ported from codeburn (MIT). See docs/ARCHITECTURE.md Related Projects.
//! Cursor session capture.
//!
//! Cursor stores chat history inside a single SQLite file
//! (`state.vscdb`) under platform-specific paths. The chat protocol writes
//! one row per "bubble" into a key/value table called `cursorDiskKV` with
//! keys shaped like `bubbleId:<conversationId>:<bubbleId>` and `value` as a
//! JSON blob.
//!
//! [`parse_state_db`] opens the DB read-only, filters bubbles to the last
//! [`LOOKBACK_DAYS`] days, applies a ROWID cap on huge databases, and groups
//! bubbles by `conversationId` into one [`TrailRecord`] each.

use crate::record::{AgentKind, Message, MessageRole, TrailRecord};
use crate::{Result, TrailError};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OpenFlags, params};
use std::path::{Path, PathBuf};

/// Conversations older than this floor are skipped. Mirrors codeburn's policy
/// so very long-lived Cursor installs don't pay JSON-decode cost on archived
/// sessions.
pub const LOOKBACK_DAYS: i64 = 180;

/// Hard cap on rows scanned. Above this we apply a ROWID cutoff so the JSON
/// scan stays bounded even on multi-GB Cursor DBs. Codeburn used the same
/// number after observing 30s+ stalls without it.
const MAX_BUBBLES: i64 = 250_000;

/// Per-message text preview cap (in characters). Long bubbles get truncated;
/// the trail layer is for shape and signal, not full transcript replay.
const TEXT_PREVIEW_CHARS: usize = 500;

/// Default location of Cursor's `state.vscdb` for the running platform.
///
/// Returns `None` when `$HOME` is not set or the platform is not recognized.
pub fn default_state_db_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    if cfg!(target_os = "macos") {
        Some(
            home.join("Library")
                .join("Application Support")
                .join("Cursor")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb"),
        )
    } else if cfg!(target_os = "windows") {
        // On Windows %APPDATA% is the canonical home; we still look at HOME
        // for cross-shell parity (msys, git-bash). Callers that need strict
        // Windows resolution should pass an explicit path.
        Some(
            home.join("AppData")
                .join("Roaming")
                .join("Cursor")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb"),
        )
    } else {
        Some(
            home.join(".config")
                .join("Cursor")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb"),
        )
    }
}

/// Parse a Cursor `state.vscdb` into one [`TrailRecord`] per conversation.
///
/// Errors:
/// - [`TrailError::Parse`] when `cursorDiskKV` is missing or has no
///   `bubbleId:%` rows.
/// - [`TrailError::Parse`] for any underlying SQLite error.
pub fn parse_state_db(path: &Path) -> Result<Vec<TrailRecord>> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| TrailError::Parse(format!("open cursor db: {e}")))?;

    if !schema_looks_like_cursor(&conn)? {
        return Err(TrailError::Parse("cursor schema not detected".to_string()));
    }

    let rows = read_bubble_rows(&conn)?;
    Ok(group_into_records(rows))
}

/// True iff the open connection has a `cursorDiskKV` table containing at
/// least one row whose key matches `bubbleId:%`.
fn schema_looks_like_cursor(conn: &Connection) -> Result<bool> {
    // First check the table exists at all — sqlite_master is cheap and
    // avoids the `no such table` error path.
    let table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='cursorDiskKV'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| TrailError::Parse(format!("schema probe: {e}")))?;
    if table_exists == 0 {
        return Ok(false);
    }

    let bubble_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM cursorDiskKV WHERE key LIKE 'bubbleId:%' LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| TrailError::Parse(format!("schema probe: {e}")))?;
    Ok(bubble_count > 0)
}

/// One materialized row from `cursorDiskKV` after we've decoded the JSON
/// blob. We sort/group these in [`group_into_records`].
struct BubbleRow {
    rowid: i64,
    conversation_id: String,
    model: Option<String>,
    created_at: Option<DateTime<Utc>>,
    text: String,
    bubble_type: i64,
}

fn read_bubble_rows(conn: &Connection) -> Result<Vec<BubbleRow>> {
    let total = bubble_total(conn)?;
    let cutoff = if total > MAX_BUBBLES {
        rowid_cutoff(conn)?
    } else {
        0
    };

    // SQL preserves ROWID ascending order so we get insertion order naturally.
    // The ROWID cutoff is spliced in only when we're capping; otherwise the
    // query is identical to the un-capped path.
    let (sql, params_vec): (&'static str, Vec<rusqlite::types::Value>) = if cutoff > 0 {
        (
            "SELECT ROWID, value FROM cursorDiskKV \
             WHERE key LIKE 'bubbleId:%' AND ROWID >= ?1 \
             ORDER BY ROWID ASC",
            vec![cutoff.into()],
        )
    } else {
        (
            "SELECT ROWID, value FROM cursorDiskKV \
             WHERE key LIKE 'bubbleId:%' \
             ORDER BY ROWID ASC",
            vec![],
        )
    };

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| TrailError::Parse(format!("prepare bubble query: {e}")))?;
    let params_ref: Vec<&dyn rusqlite::ToSql> = params_vec
        .iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();
    let mut rows = stmt
        .query(params_ref.as_slice())
        .map_err(|e| TrailError::Parse(format!("execute bubble query: {e}")))?;

    let floor = Utc::now() - Duration::days(LOOKBACK_DAYS);
    let mut out: Vec<BubbleRow> = Vec::new();

    while let Some(row) = rows
        .next()
        .map_err(|e| TrailError::Parse(format!("read bubble row: {e}")))?
    {
        let rowid: i64 = row
            .get(0)
            .map_err(|e| TrailError::Parse(format!("read rowid: {e}")))?;
        // value is a TEXT or BLOB JSON payload. Try TEXT first; fall back to
        // bytes for the BLOB path Cursor occasionally writes.
        let json_str: String = match row.get::<_, String>(1) {
            Ok(s) => s,
            Err(_) => match row.get::<_, Vec<u8>>(1) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(_) => continue,
                },
                Err(_) => continue,
            },
        };

        let Some(parsed) = parse_bubble_json(&json_str) else {
            continue;
        };

        if let Some(ts) = parsed.created_at
            && ts < floor
        {
            continue;
        }

        out.push(BubbleRow {
            rowid,
            conversation_id: parsed.conversation_id,
            model: parsed.model,
            created_at: parsed.created_at,
            text: parsed.text,
            bubble_type: parsed.bubble_type,
        });
    }

    Ok(out)
}

fn bubble_total(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM cursorDiskKV WHERE key LIKE 'bubbleId:%'",
        [],
        |r| r.get(0),
    )
    .map_err(|e| TrailError::Parse(format!("count bubbles: {e}")))
}

fn rowid_cutoff(conn: &Connection) -> Result<i64> {
    // Mirror codeburn's "smallest rowid in the most-recent N rows" cap.
    conn.query_row(
        "SELECT MIN(rid) FROM ( \
             SELECT ROWID rid FROM cursorDiskKV \
             WHERE key LIKE 'bubbleId:%' \
             ORDER BY ROWID DESC LIMIT ?1 \
         )",
        params![MAX_BUBBLES],
        |r| r.get(0),
    )
    .map_err(|e| TrailError::Parse(format!("compute rowid cutoff: {e}")))
}

struct ParsedBubble {
    conversation_id: String,
    model: Option<String>,
    created_at: Option<DateTime<Utc>>,
    text: String,
    bubble_type: i64,
}

fn parse_bubble_json(s: &str) -> Option<ParsedBubble> {
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    let conversation_id = v.get("conversationId")?.as_str()?.to_string();
    let model = v
        .get("modelInfo")
        .and_then(|m| m.get("modelName"))
        .and_then(|n| n.as_str())
        .map(str::to_string);
    let created_at = v
        .get("createdAt")
        .and_then(|c| c.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    let raw_text = v.get("text").and_then(|t| t.as_str()).unwrap_or("");
    let text = truncate_chars(raw_text, TEXT_PREVIEW_CHARS);
    let bubble_type = v.get("type").and_then(|t| t.as_i64()).unwrap_or(0);

    Some(ParsedBubble {
        conversation_id,
        model,
        created_at,
        text,
        bubble_type,
    })
}

fn truncate_chars(s: &str, max: usize) -> String {
    let mut out = String::with_capacity(s.len().min(max * 4));
    for (i, c) in s.chars().enumerate() {
        if i >= max {
            break;
        }
        out.push(c);
    }
    out
}

fn group_into_records(mut rows: Vec<BubbleRow>) -> Vec<TrailRecord> {
    // Stable sort by ROWID — Cursor writes bubbles in arrival order so this
    // recovers the conversation timeline.
    rows.sort_by_key(|r| r.rowid);

    let mut by_conv: std::collections::BTreeMap<String, Vec<BubbleRow>> =
        std::collections::BTreeMap::new();
    for row in rows {
        by_conv
            .entry(row.conversation_id.clone())
            .or_default()
            .push(row);
    }

    let mut records = Vec::with_capacity(by_conv.len());
    for (session_id, bubbles) in by_conv {
        if bubbles.is_empty() {
            continue;
        }
        let started_at = bubbles
            .iter()
            .find_map(|b| b.created_at)
            .unwrap_or_else(Utc::now);
        let ended_at = bubbles
            .iter()
            .rev()
            .find_map(|b| b.created_at)
            .unwrap_or(started_at);
        let model = bubbles
            .iter()
            .find_map(|b| b.model.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let messages: Vec<Message> = bubbles
            .iter()
            .filter(|b| !b.text.is_empty())
            .map(|b| Message {
                role: if b.bubble_type == 1 {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                },
                timestamp: b.created_at.unwrap_or(started_at),
                text: b.text.clone(),
            })
            .collect();

        records.push(TrailRecord {
            session_id,
            agent: AgentKind::Cursor,
            model,
            started_at,
            ended_at,
            // The DB alone does not name a working directory; the watcher
            // layer is responsible for attributing the conversation to a repo.
            repo_path: PathBuf::new(),
            messages,
            files_touched: Vec::new(),
            tool_invocations: Vec::new(),
        });
    }

    records
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn truncate_chars_caps_unicode_safely() {
        let s = "a".repeat(600);
        let out = truncate_chars(&s, 500);
        assert_eq!(out.chars().count(), 500);
    }

    #[test]
    fn truncate_chars_passes_short_strings_through() {
        assert_eq!(truncate_chars("hi", 500), "hi");
    }

    #[test]
    fn parse_bubble_json_handles_missing_fields() {
        let json = r#"{"conversationId":"c","text":"x"}"#;
        let p = parse_bubble_json(json).unwrap();
        assert_eq!(p.conversation_id, "c");
        assert_eq!(p.text, "x");
        assert!(p.model.is_none());
        assert_eq!(p.bubble_type, 0);
    }

    #[test]
    fn parse_bubble_json_rejects_missing_conversation_id() {
        assert!(parse_bubble_json(r#"{"text":"x"}"#).is_none());
    }

    #[test]
    fn default_state_db_path_returns_some_when_home_set() {
        // We do not assume HOME is set in CI; only verify the function does
        // not panic and produces an absolute-ish path when HOME is present.
        if std::env::var_os("HOME").is_some() {
            let p = default_state_db_path().expect("path with HOME set");
            assert!(p.ends_with("state.vscdb"));
        }
    }
}
