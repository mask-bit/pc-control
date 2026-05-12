use crate::models::{AssistantSettings, ChatMessage, LogEntry, Routine};
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
            "#,
        )
        .map_err(|err| err.to_string())
    }

    pub fn get_settings(&self) -> Result<AssistantSettings, String> {
        match self.get_setting("assistant_settings")? {
            Some(raw) => serde_json::from_str(&raw).map_err(|err| err.to_string()),
            None => Ok(AssistantSettings::default()),
        }
    }

    pub fn save_settings(&self, settings: &AssistantSettings) -> Result<(), String> {
        self.set_setting(
            "assistant_settings",
            &serde_json::to_string(settings).map_err(|err| err.to_string())?,
        )
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
}
