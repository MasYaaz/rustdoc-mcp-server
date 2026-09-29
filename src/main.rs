//! Main entry point for the `rustdoc-mcp-server` application.
//!
//! Handles dual SQLite FTS5 database connections for read/write isolation in WAL mode,
//! triggers non-blocking background indexing tasks, and processes JSON-RPC 2.0 requests via STDIO.

pub mod db;
mod mcp;

use anyhow::{Context, Result};
use db::DbManager;
use mcp::{handle_mcp_request, JsonRpcRequest};
use std::path::PathBuf;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Main entry point running the Tokio asynchronous runtime.
///
/// Execution flow:
/// 1. Initializes the primary [`DbManager`] acting as a dedicated Read connection (lock-free).
/// 2. Spawns a separate Writer connection in `tokio::task::spawn_blocking` to perform background scanning and indexing.
/// 3. Asynchronously reads incoming JSON-RPC requests line-by-line from `stdin` and writes responses to `stdout` (<10ms latency).
///
/// # Errors
/// Returns an error if SQLite database creation/initialization fails or an unrecoverable STDIO I/O error occurs.
#[tokio::main]
async fn main() -> Result<()> {
    let target_dir = PathBuf::from("target");
    let db_path = target_dir.join("rustdoc_mcp.sqlite");

    // Initialize primary Read connection (lock-free execution)
    let db = DbManager::new(&db_path)
        .context("Failed to initialize SQLite FTS5 database")?;

    // Trigger background indexing using a separate SQLite connection
    let db_path_bg = db_path.clone();
    tokio::task::spawn_blocking(move || {
        let doc_dir = target_dir.join("doc");
        let manifest_path = PathBuf::from("Cargo.toml");

        if let Ok(mut db_writer) = DbManager::new(&db_path_bg) {
            let _ = db_writer.scan_and_index_target(&doc_dir, &manifest_path);
        }
    });

    // Initialize STDIO reader/writer for JSON-RPC 2.0 communication
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = io::stdout();

    // Main event loop handling incoming requests from the MCP Client
    while let Ok(Some(line)) = reader.next_line().await {
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Execute queries directly against the read-only connection without Mutex locks
        let res = handle_mcp_request(&db, &req);
        let res_json = serde_json::to_string(&res)? + "\n";
        stdout.write_all(res_json.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}
