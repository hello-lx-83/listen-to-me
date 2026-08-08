use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};

use crate::core::models::{
    AppSettings, DictionaryEntry, DictionaryEntryInput, HistoryRecord, RewriteMode,
};

pub struct SqliteStore {
    connection: Mutex<Connection>,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create app data directory: {error}"))?;
        }

        let connection = Connection::open(path)
            .map_err(|error| format!("failed to open local database: {error}"))?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA foreign_keys = ON;
                 CREATE TABLE IF NOT EXISTS settings (
                   key TEXT PRIMARY KEY NOT NULL,
                   value TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS history (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   created_at INTEGER NOT NULL,
                   mode TEXT NOT NULL,
                   transcript TEXT NOT NULL,
                   output TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_history_created_at
                   ON history(created_at DESC);
                 CREATE TABLE IF NOT EXISTS dictionary (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   source TEXT NOT NULL,
                   replacement TEXT NOT NULL,
                   category TEXT NOT NULL,
                   updated_at INTEGER NOT NULL
                 );
                 CREATE UNIQUE INDEX IF NOT EXISTS idx_dictionary_source
                   ON dictionary(source COLLATE NOCASE);",
            )
            .map_err(|error| format!("failed to migrate local database: {error}"))?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn settings(&self) -> Result<AppSettings, String> {
        let connection = self.connection()?;
        Ok(AppSettings {
            theme: setting(&connection, "theme")?.unwrap_or_else(|| "system".to_owned()),
            language: setting(&connection, "language")?.unwrap_or_else(|| "auto".to_owned()),
            rewrite_mode: parse_rewrite_mode(
                &setting(&connection, "rewrite_mode")?.unwrap_or_else(|| "clean".to_owned()),
            ),
            save_history: setting(&connection, "save_history")?
                .map(|value| value == "true")
                .unwrap_or(true),
        })
    }

    pub fn update_settings(&self, settings: &AppSettings) -> Result<(), String> {
        if !matches!(settings.theme.as_str(), "system" | "light" | "dark") {
            return Err("invalid theme setting".to_owned());
        }
        if !matches!(settings.language.as_str(), "auto" | "zh" | "en") {
            return Err("invalid language setting".to_owned());
        }

        let mode = rewrite_mode_value(settings.rewrite_mode);
        let save_history = if settings.save_history {
            "true"
        } else {
            "false"
        };
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| format!("failed to update settings: {error}"))?;
        for (key, value) in [
            ("theme", settings.theme.as_str()),
            ("language", settings.language.as_str()),
            ("rewrite_mode", mode),
            ("save_history", save_history),
        ] {
            transaction
                .execute(
                    "INSERT INTO settings(key, value) VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )
                .map_err(|error| format!("failed to persist settings: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit settings: {error}"))
    }

    pub fn add_history(
        &self,
        mode: RewriteMode,
        transcript: &str,
        output: &str,
    ) -> Result<(), String> {
        self.connection()?
            .execute(
                "INSERT INTO history(created_at, mode, transcript, output)
                 VALUES (?1, ?2, ?3, ?4)",
                params![now_millis()?, rewrite_mode_value(mode), transcript, output],
            )
            .map(|_| ())
            .map_err(|error| format!("failed to save history: {error}"))
    }

    pub fn list_history(&self, limit: u32) -> Result<Vec<HistoryRecord>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, created_at, mode, transcript, output
                 FROM history ORDER BY created_at DESC LIMIT ?1",
            )
            .map_err(|error| format!("failed to query history: {error}"))?;
        let rows = statement
            .query_map(params![i64::from(limit.min(500))], |row| {
                let mode: String = row.get(2)?;
                Ok(HistoryRecord {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    mode: parse_rewrite_mode(&mode),
                    transcript: row.get(3)?,
                    output: row.get(4)?,
                })
            })
            .map_err(|error| format!("failed to read history: {error}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to decode history: {error}"))
    }

    pub fn delete_history(&self, id: i64) -> Result<(), String> {
        self.connection()?
            .execute("DELETE FROM history WHERE id = ?1", params![id])
            .map(|_| ())
            .map_err(|error| format!("failed to delete history: {error}"))
    }

    pub fn clear_history(&self) -> Result<(), String> {
        self.connection()?
            .execute("DELETE FROM history", [])
            .map(|_| ())
            .map_err(|error| format!("failed to clear history: {error}"))
    }

    pub fn dashboard_counts(&self) -> Result<(i64, i64), String> {
        let connection = self.connection()?;
        let history = connection
            .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
            .map_err(|error| format!("failed to count history: {error}"))?;
        let dictionary = connection
            .query_row("SELECT COUNT(*) FROM dictionary", [], |row| row.get(0))
            .map_err(|error| format!("failed to count dictionary: {error}"))?;
        Ok((history, dictionary))
    }

    pub fn list_dictionary(&self) -> Result<Vec<DictionaryEntry>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, source, replacement, category, updated_at
                 FROM dictionary ORDER BY category, source COLLATE NOCASE",
            )
            .map_err(|error| format!("failed to query dictionary: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok(DictionaryEntry {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    replacement: row.get(2)?,
                    category: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|error| format!("failed to read dictionary: {error}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to decode dictionary: {error}"))
    }

    pub fn upsert_dictionary(
        &self,
        input: &DictionaryEntryInput,
    ) -> Result<DictionaryEntry, String> {
        let source = input.source.trim();
        let replacement = input.replacement.trim();
        let category = input.category.trim();
        if source.is_empty() || replacement.is_empty() || category.is_empty() {
            return Err("dictionary fields cannot be empty".to_owned());
        }

        let timestamp = now_millis()?;
        let connection = self.connection()?;
        let id = if let Some(id) = input.id {
            connection
                .execute(
                    "UPDATE dictionary SET source = ?1, replacement = ?2,
                     category = ?3, updated_at = ?4 WHERE id = ?5",
                    params![source, replacement, category, timestamp, id],
                )
                .map_err(map_dictionary_write_error)?;
            id
        } else {
            connection
                .execute(
                    "INSERT INTO dictionary(source, replacement, category, updated_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![source, replacement, category, timestamp],
                )
                .map_err(map_dictionary_write_error)?;
            connection.last_insert_rowid()
        };

        Ok(DictionaryEntry {
            id,
            source: source.to_owned(),
            replacement: replacement.to_owned(),
            category: category.to_owned(),
            updated_at: timestamp,
        })
    }

    pub fn delete_dictionary(&self, id: i64) -> Result<(), String> {
        self.connection()?
            .execute("DELETE FROM dictionary WHERE id = ?1", params![id])
            .map(|_| ())
            .map_err(|error| format!("failed to delete dictionary entry: {error}"))
    }

    fn connection(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.connection
            .lock()
            .map_err(|_| "local database is unavailable".to_owned())
    }
}

fn setting(connection: &Connection, key: &str) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("failed to read setting: {error}"))
}

fn rewrite_mode_value(mode: RewriteMode) -> &'static str {
    match mode {
        RewriteMode::Raw => "raw",
        RewriteMode::Clean => "clean",
        RewriteMode::Article => "article",
        RewriteMode::Structured => "structured",
    }
}

fn parse_rewrite_mode(value: &str) -> RewriteMode {
    match value {
        "raw" => RewriteMode::Raw,
        "article" => RewriteMode::Article,
        "structured" => RewriteMode::Structured,
        _ => RewriteMode::Clean,
    }
}

fn now_millis() -> Result<i64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the Unix epoch".to_owned())?;
    i64::try_from(duration.as_millis()).map_err(|_| "system clock value is too large".to_owned())
}

fn map_dictionary_write_error(error: rusqlite::Error) -> String {
    if matches!(error, rusqlite::Error::SqliteFailure(_, Some(ref message)) if message.contains("UNIQUE"))
    {
        "a dictionary entry with this source already exists".to_owned()
    } else {
        format!("failed to save dictionary entry: {error}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_store() -> SqliteStore {
        SqliteStore::open(Path::new(":memory:")).expect("open in-memory database")
    }

    #[test]
    fn persists_settings_history_and_dictionary() {
        let store = memory_store();
        let settings = AppSettings {
            theme: "dark".to_owned(),
            language: "zh".to_owned(),
            rewrite_mode: RewriteMode::Structured,
            save_history: false,
        };
        store.update_settings(&settings).expect("save settings");
        assert_eq!(store.settings().expect("read settings").theme, "dark");

        store
            .add_history(RewriteMode::Clean, "原文", "结果")
            .expect("save history");
        assert_eq!(store.list_history(10).expect("list history").len(), 1);

        store
            .upsert_dictionary(&DictionaryEntryInput {
                id: None,
                source: "codex".to_owned(),
                replacement: "Codex".to_owned(),
                category: "产品".to_owned(),
            })
            .expect("save dictionary");
        assert_eq!(store.list_dictionary().expect("list dictionary").len(), 1);
    }
}
