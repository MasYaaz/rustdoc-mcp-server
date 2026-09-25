//! Modul penanganan protokol Model Context Protocol (MCP) dan JSON-RPC.
//!
//! Modul ini mengelola komunikasi JSON-RPC 2.0 antara MCP Client (seperti Claude / AI Agent)
//! dan server, serta memetakan eksekusi `tools/call` ke metode query database SQLite yang menghasilkan
//! output berformat Markdown (`.md`).

pub mod tools;

use crate::db::DbManager;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Struktur DTO untuk merepresentasikan permintaan (Request) JSON-RPC 2.0 dari client.
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct JsonRpcRequest {
    /// Versi protokol JSON-RPC (biasanya `"2.0"`).
    pub jsonrpc: String,
    /// ID unik permintaan dari client (bisa berupa angka, string, atau null).
    pub id: Option<Value>,
    /// Nama metode JSON-RPC yang dipanggil (misal: `"initialize"`, `"tools/list"`, `"tools/call"`).
    pub method: String,
    /// Parameter tambahan yang dikirimkan dalam permintaan.
    pub params: Option<Value>,
}

/// Struktur DTO untuk merepresentasikan tanggapan (Response) JSON-RPC 2.0 ke client.
#[derive(Serialize)]
pub struct JsonRpcResponse {
    /// Versi protokol JSON-RPC (`"2.0"`).
    pub jsonrpc: String,
    /// ID permintaan yang sesuai dengan ID pada `JsonRpcRequest`.
    pub id: Option<Value>,
    /// Objek hasil eksekusi jika permintaan berhasil.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Objek error jika terjadi kegagalan pemrosesan JSON-RPC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

/// Memproses permintaan JSON-RPC yang masuk dan mengembalikan tanggapan `JsonRpcResponse`.
///
/// Fungsi ini menangani tiga metode utama sesuai spesifikasi MCP:
/// - `"initialize"`: Inisialisasi awal server dan pengiriman kapabilitas.
/// - `"tools/list"`: Mengembalikan daftar seluruh tool MCP yang tersedia dari modul `tools`.
/// - `"tools/call"`: Mengeksekusi fungsi query Markdown di `DbManager` berdasarkan nama tool dan argumen yang diberikan.
///
/// # Arguments
/// * `db` - Referensi ke `DbManager` untuk melakukan query data dokumentasi.
/// * `req` - Struktur `JsonRpcRequest` yang berisi payload permintaan dari client.
///
/// # Returns
/// Mengembalikan objek `JsonRpcResponse` berisi teks Markdown atau pesan error JSON-RPC.
pub fn handle_mcp_request(db: &DbManager, req: &JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: req.id.clone(),
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "rustdoc-mcp-server", "version": "0.1.0" }
            })),
            error: None,
        },
        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: req.id.clone(),
            result: Some(tools::get_tools_list()),
            error: None,
        },
        "tools/call" => {
            let params = match req.params.as_ref() {
                Some(p) => p,
                None => {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id: req.id.clone(),
                        result: None,
                        error: Some(json!({ "code": -32602, "message": "Invalid params" })),
                    }
                }
            };

            let name = params["name"].as_str().unwrap_or("");
            let args = &params["arguments"];

            let content = match name {
                // --- 1. Eksplorasi & Discovery ---
                "get_crate_list" => match db.get_crate_list() {
                    Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                    Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true }),
                },
                "search_symbols" => {
                    let query = args["query"].as_str().unwrap_or("");
                    let kind = args["kind"].as_str();
                    match db.search(query, kind) {
                        Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true }),
                    }
                }
                "get_module_contents" => {
                    let module_path = args["module_path"].as_str().unwrap_or("");
                    match db.get_module_contents(module_path) {
                        Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true }),
                    }
                }

                // --- 2. Pembacaan Detail Tipe & API ---
                "get_type_definition" => {
                    let full_path = args["full_path"].as_str().unwrap_or("");
                    match db.get_type_definition(full_path) {
                        Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true }),
                    }
                }
                "get_function_signature" => {
                    let full_path = args["full_path"].as_str().unwrap_or("");
                    match db.get_function_signature(full_path) {
                        Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true }),
                    }
                }
                "get_associated_methods" => {
                    let struct_path = args["struct_path"].as_str().unwrap_or("");
                    match db.get_associated_methods(struct_path) {
                        Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true }),
                    }
                }
                "get_struct_fields" => {
                    let struct_path = args["struct_path"].as_str().unwrap_or("");
                    match db.get_struct_fields(struct_path) {
                        Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true }),
                    }
                }

                // --- 3. Relasi, Contoh Kode & Advanced Metadata ---
                "get_trait_impls" => {
                    let struct_path = args["struct_path"].as_str().unwrap_or("");
                    match db.get_trait_impls(struct_path) {
                        Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true }),
                    }
                }
                "search_examples" => {
                    let full_path = args["full_path"].as_str().unwrap_or("");
                    match db.search_examples(full_path) {
                        Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true }),
                    }
                }
                "get_reexports" => {
                    let full_path = args["full_path"].as_str().unwrap_or("");
                    match db.get_reexports(full_path) {
                        Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true }),
                    }
                }

                _ => json!({ "content": [{ "type": "text", "text": "Unknown tool" }], "isError": true }),
            };

            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: req.id.clone(),
                result: Some(content),
                error: None,
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: req.id.clone(),
            result: None,
            error: Some(json!({ "code": -32601, "message": "Method not found" })),
        },
    }
}
