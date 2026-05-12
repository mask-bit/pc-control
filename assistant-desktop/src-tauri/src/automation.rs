use crate::models::LogEntry;
use crate::storage::Database;
use chrono::{Local, Utc};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tauri::AppHandle;
use uuid::Uuid;

pub fn start_worker(_app: AppHandle, db: Arc<Database>) {
    std::thread::spawn(move || {
        let mut seen_processes: Vec<String> = Vec::new();
        let mut last_schedule_minute = String::new();

        loop {
            if let Ok(routines) = db.list_routines() {
                let now = Local::now().format("%H:%M").to_string();
                if now != last_schedule_minute {
                    for routine in routines.iter().filter(|item| item.enabled && item.trigger.kind == "schedule") {
                        if routine.trigger.value.as_deref() == Some(now.as_str()) {
                            let _ = db.add_log(&LogEntry {
                                id: Uuid::new_v4().to_string(),
                                level: "info".to_string(),
                                module: "automation".to_string(),
                                message: format!("Gatilho de horario pronto: {}", routine.name),
                                created_at: Utc::now().to_rfc3339(),
                            });
                        }
                    }
                    last_schedule_minute = now;
                }

                let mut system = System::new_all();
                system.refresh_processes();
                let current: Vec<String> = system
                    .processes()
                    .values()
                    .map(|process| process.name().to_string_lossy().to_lowercase())
                    .collect();

                for routine in routines.iter().filter(|item| item.enabled && item.trigger.kind == "app_open") {
                    if let Some(value) = routine.trigger.value.as_deref() {
                        let target = value.to_lowercase();
                        let is_open = current.iter().any(|name| name.contains(&target));
                        let was_open = seen_processes.iter().any(|name| name.contains(&target));
                        if is_open && !was_open {
                            let _ = db.add_log(&LogEntry {
                                id: Uuid::new_v4().to_string(),
                                level: "info".to_string(),
                                module: "automation".to_string(),
                                message: format!("Aplicativo detectado para rotina: {}", routine.name),
                                created_at: Utc::now().to_rfc3339(),
                            });
                        }
                    }
                }
                seen_processes = current;
            }
            std::thread::sleep(Duration::from_secs(30));
        }
    });
}
