//! SQLite database and FTS5 full-text search management module.
//!
//! This module is responsible for initializing SQLite connections, creating the `items`
//! table schema, virtual search table (`items_fts`), and managing automatic sync triggers.

pub mod indexer;
pub mod queries;

use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

/// Manages SQLite connections and database operations for storing Rust documentation symbols.
pub struct DbManager {
    /// Active SQLite connection.
    pub conn: Connection,
}

impl DbManager {
    /// Creates or opens a SQLite database at the specified `db_path`.
    ///
    /// Automatically creates parent directories if missing, sets PRAGMA performance configurations
    /// (WAL mode), initializes the `items` table, `items_fts` (FTS5) table, and synchronization triggers.
    ///
    /// # Arguments
    /// * `db_path` - File path where the `.sqlite` database file is stored.
    ///
    /// # Errors
    /// Returns an error if directory creation fails, opening the database fails,
    /// or initializing the SQL batch schema encounters an issue.
    pub fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;

        // PRAGMA performance tuning for fast concurrent access (< 10ms)
        conn.execute_batch(
            "
                PRAGMA journal_mode = WAL;
                PRAGMA synchronous = NORMAL;
                PRAGMA temp_store = MEMORY;
                PRAGMA busy_timeout = 5000;
                ",
        )?;

        conn.execute_batch(
            "
                CREATE TABLE IF NOT EXISTS items (
                    id TEXT PRIMARY KEY,
                    crate_name TEXT,
                    crate_version TEXT,
                    kind TEXT,
                    name TEXT,
                    full_path TEXT,
                    docs TEXT,
                    signature TEXT,
                    parent_path TEXT
                );

                CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
                    name,
                    full_path,
                    docs,
                    content='items',
                    content_rowid='rowid'
                );

                CREATE TRIGGER IF NOT EXISTS items_ai AFTER INSERT ON items BEGIN
                    INSERT INTO items_fts(rowid, name, full_path, docs)
                    VALUES (new.rowid, new.name, new.full_path, new.docs);
                END;

                CREATE TRIGGER IF NOT EXISTS items_ad AFTER DELETE ON items BEGIN
                    INSERT INTO items_fts(items_fts, rowid, name, full_path, docs)
                    VALUES ('delete', old.rowid, old.name, old.full_path, old.docs);
                END;

                CREATE TRIGGER IF NOT EXISTS items_au AFTER UPDATE ON items BEGIN
                    INSERT INTO items_fts(items_fts, rowid, name, full_path, docs)
                    VALUES ('delete', old.rowid, old.name, old.full_path, old.docs);
                    INSERT INTO items_fts(rowid, name, full_path, docs)
                    VALUES (new.rowid, new.name, new.full_path, new.docs);
                END;
                ",
        )?;

        Ok(Self { conn })
    }
}
