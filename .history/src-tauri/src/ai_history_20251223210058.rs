use crate::utils::get_config_dir;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const DB_FILE_NAME: &str = "ai_history.sqlite";
const MAX_SESSIONS: usize = 100;
const MAX_MESSAGES_PER_SESSION: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i64,
    pub role: String,
    pub text: String,
    pub timestamp: Option<i64>,
    pub model: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionSummary {
    pub id: String,
    pub title: String,
    pub model: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistorySummary {
    pub sessions: Vec<ChatSessionSummary>,
    pub current_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePatch {
    pub text: Option<String>,
    pub timestamp: Option<i64>,
    pub model: Option<String>,
    pub usage: Option<TokenUsage>,
}

fn db_path() -> std::path::PathBuf {
    get_config_dir().join(DB_FILE_NAME)
}

fn open_db() -> Result<Connection, String> {
    let path = db_path();
    let conn = Connection::open(path).map_err(|e| format!("打开数据库失败: {}", e))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("设置 foreign_keys 失败: {}", e))?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
          id TEXT PRIMARY KEY,
          title TEXT NOT NULL,
          model TEXT NOT NULL,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS messages (
          session_id TEXT NOT NULL,
          id INTEGER NOT NULL,
          role TEXT NOT NULL,
          text TEXT NOT NULL,
          timestamp INTEGER NOT NULL,
          model TEXT,
          usage_json TEXT,
          PRIMARY KEY (session_id, id),
          FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);

        CREATE TABLE IF NOT EXISTS meta (
          key TEXT PRIMARY KEY,
          value TEXT
        );
        "#,
    )
    .map_err(|e| format!("初始化数据库失败: {}", e))?;

    Ok(conn)
}

fn now_ms() -> i64 {
    crate::service::events::current_timestamp() as i64
}

fn set_current_session(conn: &Connection, session_id: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES('current_session_id', ?) \
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![session_id],
    )
    .map_err(|e| format!("写入 current_session_id 失败: {}", e))?;
    Ok(())
}

fn prune_sessions(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "DELETE FROM sessions WHERE id NOT IN (SELECT id FROM sessions ORDER BY updated_at DESC LIMIT ?1)",
        params![MAX_SESSIONS as i64],
    )
    .map_err(|e| format!("裁剪会话失败: {}", e))?;
    Ok(())
}

fn prune_messages(conn: &Connection, session_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM messages WHERE session_id = ?1 AND id NOT IN \
         (SELECT id FROM messages WHERE session_id = ?1 ORDER BY id DESC LIMIT ?2)",
        params![session_id, MAX_MESSAGES_PER_SESSION as i64],
    )
    .map_err(|e| format!("裁剪消息失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn ai_history_load() -> Result<ChatHistorySummary, String> {
    let conn = open_db()?;

    let current_session_id: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key='current_session_id' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("读取 current_session_id 失败: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.title, s.model, s.created_at, s.updated_at, \
             (SELECT COUNT(1) FROM messages m WHERE m.session_id = s.id) AS message_count \
             FROM sessions s ORDER BY s.updated_at DESC LIMIT ?1",
        )
        .map_err(|e| format!("查询会话列表失败: {}", e))?;

    let rows = stmt
        .query_map(params![MAX_SESSIONS as i64], |row| {
            Ok(ChatSessionSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                model: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                message_count: row.get(5)?,
            })
        })
        .map_err(|e| format!("读取会话列表失败: {}", e))?;

    let mut sessions = Vec::new();
    for r in rows {
        sessions.push(r.map_err(|e| format!("解析会话列表失败: {}", e))?);
    }

    Ok(ChatHistorySummary {
        sessions,
        current_session_id,
    })
}

#[tauri::command]
pub fn ai_history_get_session(session_id: String) -> Result<Option<ChatSession>, String> {
    let conn = open_db()?;

    let session_row: Option<(String, String, String, i64, i64)> = conn
        .query_row(
            "SELECT id, title, model, created_at, updated_at FROM sessions WHERE id = ?1 LIMIT 1",
            params![session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(|e| format!("查询会话失败: {}", e))?;

    let (id, title, model, created_at, updated_at) = match session_row {
        Some(v) => v,
        None => return Ok(None),
    };

    let mut stmt = conn
        .prepare(
            "SELECT id, role, text, timestamp, model, usage_json \
             FROM messages WHERE session_id = ?1 ORDER BY id ASC LIMIT ?2",
        )
        .map_err(|e| format!("查询消息失败: {}", e))?;

    let rows = stmt
        .query_map(params![&id, MAX_MESSAGES_PER_SESSION as i64], |row| {
            let usage_json: Option<String> = row.get(5)?;
            let usage = match usage_json {
                Some(s) if !s.trim().is_empty() => serde_json::from_str::<TokenUsage>(&s).ok(),
                _ => None,
            };
            Ok(ChatMessage {
                id: row.get(0)?,
                role: row.get(1)?,
                text: row.get(2)?,
                timestamp: Some(row.get::<_, i64>(3)?),
                model: row.get(4)?,
                usage,
            })
        })
        .map_err(|e| format!("读取消息失败: {}", e))?;

    let mut messages = Vec::new();
    for r in rows {
        messages.push(r.map_err(|e| format!("解析消息失败: {}", e))?);
    }

    Ok(Some(ChatSession {
        id,
        title,
        model,
        messages,
        created_at,
        updated_at,
    }))
}

#[tauri::command]
pub fn ai_history_upsert_session(session: ChatSession) -> Result<(), String> {
    let mut conn = open_db()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启事务失败: {}", e))?;

    tx.execute(
        "INSERT INTO sessions(id, title, model, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(id) DO UPDATE SET title=excluded.title, model=excluded.model, \
         created_at=excluded.created_at, updated_at=excluded.updated_at",
        params![
            session.id,
            session.title,
            session.model,
            session.created_at,
            session.updated_at
        ],
    )
    .map_err(|e| format!("写入会话失败: {}", e))?;

    tx.execute(
        "DELETE FROM messages WHERE session_id = ?1",
        params![&session.id],
    )
    .map_err(|e| format!("清理旧消息失败: {}", e))?;

    let now = now_ms();
    for msg in session.messages.iter().take(MAX_MESSAGES_PER_SESSION) {
        let ts = msg.timestamp.unwrap_or(now);
        let usage_json = msg
            .usage
            .as_ref()
            .map(|u| serde_json::to_string(u).unwrap_or_default());
        tx.execute(
            "INSERT OR REPLACE INTO messages(session_id, id, role, text, timestamp, model, usage_json) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &session.id,
                msg.id,
                msg.role,
                msg.text,
                ts,
                msg.model,
                usage_json
            ],
        )
        .map_err(|e| format!("写入消息失败: {}", e))?;
    }

    set_current_session(&tx, &session.id)?;
    prune_sessions(&tx)?;
    prune_messages(&tx, &session.id)?;

    tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn ai_history_add_message(session_id: String, message: ChatMessage) -> Result<(), String> {
    let mut conn = open_db()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启事务失败: {}", e))?;

    let now = now_ms();
    let exists: bool = tx
        .query_row(
            "SELECT 1 FROM sessions WHERE id = ?1 LIMIT 1",
            params![&session_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("查询会话是否存在失败: {}", e))?
        .unwrap_or(false);

    if !exists {
        tx.execute(
            "INSERT INTO sessions(id, title, model, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![&session_id, "新会话", message.model.clone().unwrap_or_else(|| "unknown".to_string()), now, now],
        )
        .map_err(|e| format!("补全会话失败: {}", e))?;
    }

    let ts = message.timestamp.unwrap_or(now);
    let usage_json = message
        .usage
        .as_ref()
        .map(|u| serde_json::to_string(u).unwrap_or_default());

    tx.execute(
        "INSERT OR REPLACE INTO messages(session_id, id, role, text, timestamp, model, usage_json) \
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            &session_id,
            message.id,
            message.role,
            message.text,
            ts,
            message.model,
            usage_json
        ],
    )
    .map_err(|e| format!("写入消息失败: {}", e))?;

    tx.execute(
        "UPDATE sessions SET updated_at = ?2 WHERE id = ?1",
        params![&session_id, now],
    )
    .map_err(|e| format!("更新会话时间失败: {}", e))?;

    set_current_session(&tx, &session_id)?;
    prune_messages(&tx, &session_id)?;
    prune_sessions(&tx)?;

    tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn ai_history_update_message(
    session_id: String,
    message_id: i64,
    patch: MessagePatch,
) -> Result<(), String> {
    let mut conn = open_db()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启事务失败: {}", e))?;

    let row: Option<(String, i64, String, String, i64, Option<String>, Option<String>)> = tx
        .query_row(
            "SELECT session_id, id, role, text, timestamp, model, usage_json \
             FROM messages WHERE session_id = ?1 AND id = ?2 LIMIT 1",
            params![&session_id, message_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .optional()
        .map_err(|e| format!("查询消息失败: {}", e))?;

    let (sid, mid, role, old_text, old_ts, old_model, old_usage_json) = match row {
        Some(v) => v,
        None => return Ok(()),
    };

    let new_text = patch.text.unwrap_or(old_text);
    let new_ts = patch.timestamp.unwrap_or(old_ts);
    let new_model = patch.model.or(old_model);
    let new_usage_json = match patch.usage {
        Some(u) => Some(serde_json::to_string(&u).unwrap_or_default()),
        None => old_usage_json,
    };

    tx.execute(
        "UPDATE messages SET text = ?3, timestamp = ?4, model = ?5, usage_json = ?6 \
         WHERE session_id = ?1 AND id = ?2",
        params![&sid, mid, new_text, new_ts, new_model, new_usage_json],
    )
    .map_err(|e| format!("更新消息失败: {}", e))?;

    let now = now_ms();
    tx.execute(
        "UPDATE sessions SET updated_at = ?2 WHERE id = ?1",
        params![&session_id, now],
    )
    .map_err(|e| format!("更新会话时间失败: {}", e))?;

    set_current_session(&tx, &session_id)?;

    tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn ai_history_delete_session(session_id: String) -> Result<(), String> {
    let mut conn = open_db()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启事务失败: {}", e))?;

    tx.execute("DELETE FROM sessions WHERE id = ?1", params![&session_id])
        .map_err(|e| format!("删除会话失败: {}", e))?;

    let current_session_id: Option<String> = tx
        .query_row(
            "SELECT value FROM meta WHERE key='current_session_id' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("读取 current_session_id 失败: {}", e))?;

    if current_session_id.as_deref() == Some(session_id.as_str()) {
        let next: Option<String> = tx
            .query_row(
                "SELECT id FROM sessions ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("查询下一会话失败: {}", e))?;

        match next {
            Some(id) => set_current_session(&tx, &id)?,
            None => {
                tx.execute("DELETE FROM meta WHERE key='current_session_id'", [])
                    .map_err(|e| format!("清理 current_session_id 失败: {}", e))?;
            }
        }
    }

    tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn ai_history_clear() -> Result<(), String> {
    let mut conn = open_db()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启事务失败: {}", e))?;
    tx.execute("DELETE FROM messages", [])
        .map_err(|e| format!("清空 messages 失败: {}", e))?;
    tx.execute("DELETE FROM sessions", [])
        .map_err(|e| format!("清空 sessions 失败: {}", e))?;
    tx.execute("DELETE FROM meta", [])
        .map_err(|e| format!("清空 meta 失败: {}", e))?;
    tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;
    Ok(())
}
