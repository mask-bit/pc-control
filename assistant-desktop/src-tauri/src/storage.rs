use crate::models::{
    AssistantSettings, ChatMessage, CommandHistoryEntry, GoogleProfile, LogEntry, MemoryEntry,
    MusicFavorite, Routine, TaskState,
};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("Falha ao localizar pasta de dados: {err}"))?;
        std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let path: PathBuf = dir.join("assistente.sqlite3");
        let conn = Connection::open(path).map_err(|err| err.to_string())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS chat_messages (
                id TEXT PRIMARY KEY,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                tool_calls TEXT,
                status TEXT
            );
            CREATE TABLE IF NOT EXISTS routines (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                trigger_json TEXT NOT NULL,
                actions_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS logs (
                id TEXT PRIMARY KEY,
                level TEXT NOT NULL,
                module TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS command_history (
                id TEXT PRIMARY KEY,
                command TEXT NOT NULL,
                actions_json TEXT NOT NULL,
                assistant_reply TEXT NOT NULL,
                result_summary TEXT NOT NULL,
                status TEXT NOT NULL,
                source TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                category TEXT NOT NULL,
                pinned INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS music_favorites (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                uri TEXT,
                query TEXT,
                kind TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .map_err(|err| err.to_string())
    }

    pub fn get_settings(&self) -> Result<AssistantSettings, String> {
        match self.get_setting("assistant_settings")? {
            Some(raw) => Ok(serde_json::from_str(&raw).unwrap_or_default()),
            None => Ok(AssistantSettings::default()),
        }
    }

    pub fn save_settings(&self, settings: &AssistantSettings) -> Result<(), String> {
        self.set_setting(
            "assistant_settings",
            &serde_json::to_string(settings).map_err(|err| err.to_string())?,
        )
    }

    pub fn get_google_profile(&self) -> Result<Option<GoogleProfile>, String> {
        self.get_setting("google_profile")?
            .map(|raw| serde_json::from_str(&raw).map_err(|err| err.to_string()))
            .transpose()
    }

    pub fn save_google_profile(&self, profile: &GoogleProfile) -> Result<(), String> {
        self.set_setting(
            "google_profile",
            &serde_json::to_string(profile).map_err(|err| err.to_string())?,
        )
    }

    pub fn clear_google_profile(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute("DELETE FROM settings WHERE key = ?1", params!["google_profile"])
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn is_onboarding_complete(&self) -> Result<bool, String> {
        Ok(self
            .get_setting("onboarding_complete")?
            .as_deref()
            .map(|value| value == "true")
            .unwrap_or(false))
    }

    pub fn set_onboarding_complete(&self, complete: bool) -> Result<(), String> {
        self.set_setting("onboarding_complete", if complete { "true" } else { "false" })
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        let mut stmt = conn
            .prepare("SELECT value FROM settings WHERE key = ?1")
            .map_err(|err| err.to_string())?;
        let mut rows = stmt.query(params![key]).map_err(|err| err.to_string())?;
        if let Some(row) = rows.next().map_err(|err| err.to_string())? {
            Ok(Some(row.get::<_, String>(0).map_err(|err| err.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn list_messages(&self) -> Result<Vec<ChatMessage>, String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, role, content, created_at, tool_calls, status
                 FROM chat_messages ORDER BY created_at ASC LIMIT 200",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let raw_tools: Option<String> = row.get(4)?;
                Ok(ChatMessage {
                    id: row.get(0)?,
                    role: row.get(1)?,
                    content: row.get(2)?,
                    created_at: row.get(3)?,
                    tool_calls: raw_tools
                        .and_then(|raw| serde_json::from_str(&raw).ok()),
                    status: row.get(5)?,
                })
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn save_message(&self, message: &ChatMessage) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        let tools = message
            .tool_calls
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO chat_messages
             (id, role, content, created_at, tool_calls, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                message.id,
                message.role,
                message.content,
                message.created_at,
                tools,
                message.status
            ],
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn list_routines(&self) -> Result<Vec<Routine>, String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, enabled, trigger_json, actions_json, created_at, updated_at
                 FROM routines ORDER BY updated_at DESC",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let trigger_raw: String = row.get(3)?;
                let actions_raw: String = row.get(4)?;
                Ok(Routine {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    enabled: row.get::<_, i64>(2)? == 1,
                    trigger: serde_json::from_str(&trigger_raw).unwrap_or(crate::models::Trigger {
                        kind: "manual".to_string(),
                        label: "Manual".to_string(),
                        value: None,
                        aliases: Vec::new(),
                    }),
                    actions: serde_json::from_str(&actions_raw).unwrap_or_default(),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn save_routine(&self, routine: &Routine) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO routines
             (id, name, enabled, trigger_json, actions_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                routine.id,
                routine.name,
                if routine.enabled { 1 } else { 0 },
                serde_json::to_string(&routine.trigger).map_err(|err| err.to_string())?,
                serde_json::to_string(&routine.actions).map_err(|err| err.to_string())?,
                routine.created_at,
                routine.updated_at
            ],
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn add_log(&self, entry: &LogEntry) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT INTO logs (id, level, module, message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                entry.id,
                entry.level,
                entry.module,
                entry.message,
                entry.created_at
            ],
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn list_logs(&self) -> Result<Vec<LogEntry>, String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, level, message, created_at, module
                 FROM logs ORDER BY created_at DESC LIMIT 300",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(LogEntry {
                    id: row.get(0)?,
                    level: row.get(1)?,
                    message: row.get(2)?,
                    created_at: row.get(3)?,
                    module: row.get(4)?,
                })
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn clear_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute("DELETE FROM chat_messages", [])
            .map_err(|err| err.to_string())?;
        conn.execute("DELETE FROM logs", [])
            .map_err(|err| err.to_string())?;
        conn.execute("DELETE FROM command_history", [])
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn save_history_entry(&self, entry: &CommandHistoryEntry) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO command_history
             (id, command, actions_json, assistant_reply, result_summary, status, source, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.id,
                entry.command,
                serde_json::to_string(&entry.actions).map_err(|err| err.to_string())?,
                entry.assistant_reply,
                entry.result_summary,
                entry.status,
                entry.source,
                entry.created_at
            ],
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn list_history_entries(&self) -> Result<Vec<CommandHistoryEntry>, String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, command, actions_json, assistant_reply, result_summary, status, source, created_at
                 FROM command_history ORDER BY created_at DESC LIMIT 300",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let actions_raw: String = row.get(2)?;
                Ok(CommandHistoryEntry {
                    id: row.get(0)?,
                    command: row.get(1)?,
                    actions: serde_json::from_str(&actions_raw).unwrap_or_default(),
                    assistant_reply: row.get(3)?,
                    result_summary: row.get(4)?,
                    status: row.get(5)?,
                    source: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn get_task_state(&self) -> Result<TaskState, String> {
        match self.get_setting("task_state")? {
            Some(raw) => Ok(serde_json::from_str(&raw).unwrap_or_default()),
            None => Ok(TaskState::default()),
        }
    }

    pub fn save_task_state(&self, task_state: &TaskState) -> Result<(), String> {
        self.set_setting(
            "task_state",
            &serde_json::to_string(task_state).map_err(|err| err.to_string())?,
        )
    }

    pub fn clear_task_state(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute("DELETE FROM settings WHERE key = ?1", params!["task_state"])
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn list_memories(&self) -> Result<Vec<MemoryEntry>, String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, content, category, pinned, created_at, updated_at
                 FROM memories ORDER BY pinned DESC, updated_at DESC LIMIT 120",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    category: row.get(3)?,
                    pinned: row.get::<_, i64>(4)? == 1,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn save_memory(&self, memory: &MemoryEntry) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO memories
             (id, title, content, category, pinned, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                memory.id,
                memory.title,
                memory.content,
                memory.category,
                if memory.pinned { 1 } else { 0 },
                memory.created_at,
                memory.updated_at
            ],
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn delete_memory(&self, id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        let changed = conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])
            .map_err(|err| err.to_string())?;
        Ok(changed > 0)
    }

    #[allow(dead_code)]
    pub fn clear_memories(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute("DELETE FROM memories", [])
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn list_music_favorites(&self) -> Result<Vec<MusicFavorite>, String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, uri, query, kind, created_at
                 FROM music_favorites ORDER BY created_at DESC LIMIT 80",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(MusicFavorite {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    uri: row.get(2)?,
                    query: row.get(3)?,
                    kind: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn save_music_favorite(&self, favorite: &MusicFavorite) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO music_favorites
             (id, name, uri, query, kind, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                favorite.id,
                favorite.name,
                favorite.uri,
                favorite.query,
                favorite.kind,
                favorite.created_at
            ],
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn delete_music_favorite(&self, id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|err| err.to_string())?;
        let changed = conn
            .execute("DELETE FROM music_favorites WHERE id = ?1", params![id])
            .map_err(|err| err.to_string())?;
        Ok(changed > 0)
    }

    pub fn find_music_favorite(&self, query: &str) -> Result<Option<MusicFavorite>, String> {
        let normalized = query.trim().to_lowercase();
        if normalized.is_empty() {
            return Ok(None);
        }

        Ok(self
            .list_music_favorites()?
            .into_iter()
            .find(|favorite| {
                let name = favorite.name.to_lowercase();
                name == normalized || name.contains(&normalized) || normalized.contains(&name)
            }))
    }
}
