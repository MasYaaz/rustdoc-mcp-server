//! Modul manajemen database SQLite dan FTS5.
//!
//! Modul ini bertanggung jawab untuk inisialisasi koneksi SQLite, pembuatan skema tabel `items`,
//! tabel virtual pencarian teks penuh (`items_fts`), serta *trigger* pemutakhiran data otomatis.

pub mod indexer;
pub mod queries;

use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

/// Mengelola koneksi dan operasi database SQLite untuk penyimpanan simbol dokumentasi Rust.
pub struct DbManager {
    /// Koneksi SQLite aktif.
    pub conn: Connection,
}

impl DbManager {
    /// Membuat atau membuka database SQLite di lokasi `db_path`.
        ///
        /// Fungsi ini otomatis membuat folder direktori jika belum ada, lalu menginisialisasi
        /// konfigurasi performa PRAGMA (mode WAL), skema tabel `items`, tabel `items_fts` (FTS5),
        /// serta *trigger* sinkronisasi data.
        ///
        /// # Arguments
        /// * `db_path` - Path lokasi file database `.sqlite` akan disimpan.
        ///
        /// # Errors
        /// Mengembalikan error jika gagal membuat direktori, membuka file database,
        /// atau mengeksekusi *batch SQL* inisialisasi tabel.
        pub fn new(db_path: &Path) -> Result<Self> {
            if let Some(parent) = db_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let conn = Connection::open(db_path)?;

            // PRAGMA optimasi performa untuk akses konkurren cepat (< 10ms)
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
                ",
            )?;

            Ok(Self { conn })
        }
}
