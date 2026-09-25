//! Entry point utama untuk server MCP `rustdoc-mcp-server`.
//!
//! File ini mengelola koneksi database SQLite FTS5 terpisah untuk *read* dan *write* (WAL mode),
//! memicu *indexing background* secara non-blocking, serta memproses permintaan JSON-RPC 2.0 via STDIO.

mod db;
mod mcp;

use anyhow::{Context, Result};
use db::DbManager;
use mcp::{handle_mcp_request, JsonRpcRequest};
use std::path::PathBuf;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Fungsi entry point utama yang menjalankan runtime asinkronus Tokio.
///
/// Alur kerja:
/// 1. Menginisialisasi `DbManager` utama yang bertindak sebagai *Read Connection* (bebas lock contention).
/// 2. Memicu koneksi SQLite terpisah (*Writer Connection*) di `tokio::task::spawn_blocking` untuk melakukan pemindaian & *indexing*.
/// 3. Membaca baris permintaan JSON-RPC dari `stdin` secara asinkronus dan merespon ke `stdout` dengan latensi rendah (<10ms).
///
/// # Errors
/// Mengembalikan error jika file database SQLite gagal dibuka/dibuat atau terjadi kesalahan I/O fatal pada STDIO.
#[tokio::main]
async fn main() -> Result<()> {
    let target_dir = PathBuf::from("target");
    let db_path = target_dir.join("rustdoc_mcp.sqlite");

    // Inisialisasi koneksi utama untuk Read (bebas lock Mutex)
    let db = DbManager::new(&db_path)
        .context("Gagal menginisialisasi database SQLite FTS5")?;

    // Memicu indexing background menggunakan koneksi SQLite terpisah
    let db_path_bg = db_path.clone();
    tokio::task::spawn_blocking(move || {
        let doc_dir = target_dir.join("doc");
        let manifest_path = PathBuf::from("Cargo.toml");

        if let Ok(mut db_writer) = DbManager::new(&db_path_bg) {
            let _ = db_writer.scan_and_index_target(&doc_dir, &manifest_path);
        }
    });

    // Inisialisasi pembaca STDIO untuk komunikasi JSON-RPC
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = io::stdout();

    // Loop utama penanganan permintaan dari MCP Client
    while let Ok(Some(line)) = reader.next_line().await {
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Langsung eksekusi query tanpa mengunci Mutex
        let res = handle_mcp_request(&db, &req);
        let res_json = serde_json::to_string(&res)? + "\n";
        stdout.write_all(res_json.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}
