use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};

use crate::core::models::{
    AppSettings, DictionaryCategory, DictionaryEntry, DictionaryEntryInput, HistoryRecord,
    RewriteMode,
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
                   ON dictionary(source COLLATE NOCASE);
                 CREATE TABLE IF NOT EXISTS dictionary_categories (
                   name TEXT COLLATE NOCASE PRIMARY KEY NOT NULL,
                   created_at INTEGER NOT NULL
                 );
                 INSERT OR IGNORE INTO dictionary_categories(name, created_at) VALUES
                   ('通用', 0), ('产品', 0), ('人名', 0), ('专业术语', 0);
                 INSERT OR IGNORE INTO dictionary_categories(name, created_at)
                   SELECT DISTINCT category, 0 FROM dictionary WHERE trim(category) <> '';",
            )
            .map_err(|error| format!("failed to migrate local database: {error}"))?;
        let retention_days = setting(&connection, "history_retention_days")?
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|days| matches!(days, 7 | 30))
            .unwrap_or(30);
        prune_history_connection(&connection, retention_days)?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn settings(&self) -> Result<AppSettings, String> {
        let connection = self.connection()?;
        Ok(AppSettings {
            theme: setting(&connection, "theme")?.unwrap_or_else(|| "system".to_owned()),
            language: setting(&connection, "language")?.unwrap_or_else(|| "auto".to_owned()),
            rewrite_mode: parse_settings_rewrite_mode(
                &setting(&connection, "rewrite_mode")?.unwrap_or_else(|| "clean".to_owned()),
            ),
            save_history: setting(&connection, "save_history")?
                .map(|value| value == "true")
                .unwrap_or(true),
            history_retention_days: setting(&connection, "history_retention_days")?
                .and_then(|value| value.parse::<u32>().ok())
                .filter(|days| matches!(days, 7 | 30))
                .unwrap_or(30),
        })
    }

    pub fn update_settings(&self, settings: &AppSettings) -> Result<(), String> {
        if !matches!(settings.theme.as_str(), "system" | "light" | "dark") {
            return Err("invalid theme setting".to_owned());
        }
        if !matches!(settings.language.as_str(), "auto" | "zh" | "en") {
            return Err("invalid language setting".to_owned());
        }
        if !matches!(settings.history_retention_days, 7 | 30) {
            return Err("invalid history retention setting".to_owned());
        }

        let mode = rewrite_mode_value(settings.rewrite_mode);
        let save_history = if settings.save_history {
            "true"
        } else {
            "false"
        };
        let retention_days = settings.history_retention_days.to_string();
        let cutoff = history_cutoff(settings.history_retention_days)?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| format!("failed to update settings: {error}"))?;
        for (key, value) in [
            ("theme", settings.theme.as_str()),
            ("language", settings.language.as_str()),
            ("rewrite_mode", mode),
            ("save_history", save_history),
            ("history_retention_days", retention_days.as_str()),
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
            .execute("DELETE FROM history WHERE created_at < ?1", params![cutoff])
            .map_err(|error| format!("failed to apply history retention: {error}"))?;
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
        let mut connection = self.connection()?;
        let retention_days = setting(&connection, "history_retention_days")?
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|days| matches!(days, 7 | 30))
            .unwrap_or(30);
        let transaction = connection
            .transaction()
            .map_err(|error| format!("failed to save history: {error}"))?;
        transaction
            .execute(
                "INSERT INTO history(created_at, mode, transcript, output)
                 VALUES (?1, ?2, ?3, ?4)",
                params![now_millis()?, rewrite_mode_value(mode), transcript, output],
            )
            .map_err(|error| format!("failed to save history: {error}"))?;
        transaction
            .execute(
                "DELETE FROM history WHERE created_at < ?1",
                params![history_cutoff(retention_days)?],
            )
            .map_err(|error| format!("failed to prune history: {error}"))?;
        transaction
            .execute(
                "DELETE FROM history WHERE id NOT IN (
                   SELECT id FROM history ORDER BY created_at DESC LIMIT 1000
                 )",
                [],
            )
            .map_err(|error| format!("failed to limit history: {error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("failed to commit history: {error}"))
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
            .query_map(params![i64::from(limit.min(1_000))], |row| {
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
        let replacement = input.replacement.trim();
        let source = if input.source.trim().is_empty() {
            replacement
        } else {
            input.source.trim()
        };
        let category = input.category.trim();
        if replacement.is_empty() || category.is_empty() {
            return Err("dictionary fields cannot be empty".to_owned());
        }

        let timestamp = now_millis()?;
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT OR IGNORE INTO dictionary_categories(name, created_at) VALUES (?1, ?2)",
                params![category, timestamp],
            )
            .map_err(|error| format!("failed to save dictionary category: {error}"))?;
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

    pub fn list_dictionary_categories(&self) -> Result<Vec<DictionaryCategory>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT categories.name, COUNT(dictionary.id)
                 FROM dictionary_categories AS categories
                 LEFT JOIN dictionary ON dictionary.category = categories.name COLLATE NOCASE
                 GROUP BY categories.name
                 ORDER BY CASE WHEN categories.name = '通用' THEN 0 ELSE 1 END,
                          categories.name COLLATE NOCASE",
            )
            .map_err(|error| format!("failed to query dictionary categories: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok(DictionaryCategory {
                    name: row.get(0)?,
                    entry_count: row.get(1)?,
                })
            })
            .map_err(|error| format!("failed to read dictionary categories: {error}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to decode dictionary categories: {error}"))
    }

    pub fn create_dictionary_category(&self, name: &str) -> Result<DictionaryCategory, String> {
        let name = valid_category_name(name)?;
        self.connection()?
            .execute(
                "INSERT INTO dictionary_categories(name, created_at) VALUES (?1, ?2)",
                params![name, now_millis()?],
            )
            .map_err(map_category_write_error)?;
        Ok(DictionaryCategory {
            name: name.to_owned(),
            entry_count: 0,
        })
    }

    pub fn rename_dictionary_category(
        &self,
        old_name: &str,
        new_name: &str,
    ) -> Result<DictionaryCategory, String> {
        let old_name = valid_category_name(old_name)?;
        let new_name = valid_category_name(new_name)?;
        if old_name == "通用" {
            return Err("the general category cannot be renamed".to_owned());
        }

        let mut connection = self.connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| format!("failed to rename dictionary category: {error}"))?;
        let changed = transaction
            .execute(
                "UPDATE dictionary_categories SET name = ?1 WHERE name = ?2 COLLATE NOCASE",
                params![new_name, old_name],
            )
            .map_err(map_category_write_error)?;
        if changed == 0 {
            return Err("dictionary category does not exist".to_owned());
        }
        transaction
            .execute(
                "UPDATE dictionary SET category = ?1 WHERE category = ?2 COLLATE NOCASE",
                params![new_name, old_name],
            )
            .map_err(|error| format!("failed to move dictionary entries: {error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("failed to commit dictionary category: {error}"))?;

        let entry_count = connection
            .query_row(
                "SELECT COUNT(*) FROM dictionary WHERE category = ?1 COLLATE NOCASE",
                params![new_name],
                |row| row.get(0),
            )
            .map_err(|error| format!("failed to count dictionary entries: {error}"))?;
        Ok(DictionaryCategory {
            name: new_name.to_owned(),
            entry_count,
        })
    }

    pub fn delete_dictionary_category(&self, name: &str) -> Result<(), String> {
        let name = valid_category_name(name)?;
        if name == "通用" {
            return Err("the general category cannot be deleted".to_owned());
        }

        let mut connection = self.connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| format!("failed to delete dictionary category: {error}"))?;
        transaction
            .execute(
                "UPDATE dictionary SET category = '通用' WHERE category = ?1 COLLATE NOCASE",
                params![name],
            )
            .map_err(|error| format!("failed to move dictionary entries: {error}"))?;
        transaction
            .execute(
                "DELETE FROM dictionary_categories WHERE name = ?1 COLLATE NOCASE",
                params![name],
            )
            .map_err(|error| format!("failed to delete dictionary category: {error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("failed to commit dictionary category: {error}"))
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

fn parse_settings_rewrite_mode(value: &str) -> RewriteMode {
    match value {
        "raw" => RewriteMode::Raw,
        _ => RewriteMode::Clean,
    }
}

fn now_millis() -> Result<i64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the Unix epoch".to_owned())?;
    i64::try_from(duration.as_millis()).map_err(|_| "system clock value is too large".to_owned())
}

fn history_cutoff(retention_days: u32) -> Result<i64, String> {
    const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1_000;
    Ok(now_millis()?.saturating_sub(i64::from(retention_days) * MILLIS_PER_DAY))
}

fn prune_history_connection(connection: &Connection, retention_days: u32) -> Result<(), String> {
    connection
        .execute(
            "DELETE FROM history WHERE created_at < ?1",
            params![history_cutoff(retention_days)?],
        )
        .map_err(|error| format!("failed to apply history retention: {error}"))?;
    connection
        .execute(
            "DELETE FROM history WHERE id NOT IN (
               SELECT id FROM history ORDER BY created_at DESC LIMIT 1000
             )",
            [],
        )
        .map(|_| ())
        .map_err(|error| format!("failed to limit history: {error}"))
}

fn map_dictionary_write_error(error: rusqlite::Error) -> String {
    if matches!(error, rusqlite::Error::SqliteFailure(_, Some(ref message)) if message.contains("UNIQUE"))
    {
        "a dictionary entry with this source already exists".to_owned()
    } else {
        format!("failed to save dictionary entry: {error}")
    }
}

fn valid_category_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        Err("dictionary category cannot be empty".to_owned())
    } else if name.chars().count() > 24 {
        Err("dictionary category is too long".to_owned())
    } else {
        Ok(name)
    }
}

fn map_category_write_error(error: rusqlite::Error) -> String {
    if matches!(error, rusqlite::Error::SqliteFailure(_, Some(ref message)) if message.contains("UNIQUE"))
    {
        "dictionary category already exists".to_owned()
    } else {
        format!("failed to save dictionary category: {error}")
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
            history_retention_days: 7,
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

    #[test]
    fn legacy_structured_setting_migrates_to_smart_mode() {
        let store = memory_store();
        store
            .connection()
            .expect("database connection")
            .execute(
                "INSERT INTO settings(key, value) VALUES ('rewrite_mode', 'structured')",
                [],
            )
            .expect("save legacy setting");

        assert!(matches!(
            store.settings().expect("read settings").rewrite_mode,
            RewriteMode::Clean
        ));
    }

    #[test]
    fn categories_can_be_created_renamed_and_deleted_without_losing_entries() {
        let store = memory_store();
        store
            .create_dictionary_category("AI 与开发")
            .expect("create category");
        store
            .upsert_dictionary(&DictionaryEntryInput {
                id: None,
                source: "诶真特".to_owned(),
                replacement: "Agent".to_owned(),
                category: "AI 与开发".to_owned(),
            })
            .expect("save dictionary entry");

        let renamed = store
            .rename_dictionary_category("AI 与开发", "开发术语")
            .expect("rename category");
        assert_eq!(renamed.name, "开发术语");
        assert_eq!(renamed.entry_count, 1);
        assert_eq!(
            store.list_dictionary().expect("list dictionary")[0].category,
            "开发术语"
        );

        store
            .delete_dictionary_category("开发术语")
            .expect("delete category");
        assert_eq!(
            store.list_dictionary().expect("list dictionary")[0].category,
            "通用"
        );
        assert!(!store
            .list_dictionary_categories()
            .expect("list categories")
            .iter()
            .any(|category| category.name == "开发术语"));
    }

    #[test]
    fn changing_retention_removes_expired_history_immediately() {
        const EIGHT_DAYS_MILLIS: i64 = 8 * 24 * 60 * 60 * 1_000;
        let store = memory_store();
        store
            .add_history(RewriteMode::Clean, "旧记录", "旧记录")
            .expect("save old history");
        store
            .connection()
            .expect("database connection")
            .execute(
                "UPDATE history SET created_at = ?1",
                params![now_millis().expect("current time") - EIGHT_DAYS_MILLIS],
            )
            .expect("age history");
        store
            .add_history(RewriteMode::Clean, "新记录", "新记录")
            .expect("save recent history");

        let mut settings = store.settings().expect("read settings");
        settings.history_retention_days = 7;
        store.update_settings(&settings).expect("update retention");

        let history = store.list_history(10).expect("list history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].output, "新记录");
    }
}
