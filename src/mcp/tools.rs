//! Modul pendaftaran dan penyedia skema Tool Model Context Protocol (MCP).
//!
//! Modul ini mendefinisikan daftar seluruh tool MCP yang didukung oleh server,
//! lengkap dengan nama, deskripsi berformat Markdown, serta skema JSON Schema dari argumen inputnya (`inputSchema`).

use serde_json::json;

/// Mengembalikan objek JSON yang berisi skema dan deskripsi dari seluruh tool MCP yang tersedia.
///
/// Seluruh tool mengembalikan output berformat Markdown (`.md`) yang dioptimalkan untuk AI Agent/LLM.
///
/// Tool yang terdaftar dikategorikan ke dalam tiga kelompok utama:
/// 1. **Eksplorasi & Discovery**: `get_crate_list`, `search_symbols`, `get_module_contents`
/// 2. **Pembacaan Detail Tipe & API**: `get_type_definition`, `get_function_signature`, `get_associated_methods`, `get_struct_fields`
/// 3. **Relasi, Contoh Kode & Metadata**: `get_trait_impls`, `search_examples`, `get_reexports`
///
/// # Returns
/// Mengembalikan `serde_json::Value` yang merepresentasikan respons JSON spesifikasi MCP untuk metode `tools/list`.
pub fn get_tools_list() -> serde_json::Value {
    json!({
        "tools": [
            // --- 1. Eksplorasi & Discovery ---
            {
                "name": "get_crate_list",
                "description": "Dapatkan daftar semua crate beserta versinya yang saat ini ter-index di database (Format: Markdown List)",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "search_symbols",
                "description": "Cari fungsi, struct, enum, atau module di dokumentasi Rust menggunakan FTS5 dengan filter optional 'kind' (Format: Markdown Table)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Nama simbol / kata kunci" },
                        "kind": { "type": "string", "description": "Filter tipe simbol (misal: 'struct', 'function', 'trait', 'enum')" }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "get_module_contents",
                "description": "Dapatkan daftar isi item (struct, function, sub-module) dalam suatu module (Format: Markdown Table)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "module_path": { "type": "string", "description": "Contoh: rusqlite::types" }
                    },
                    "required": ["module_path"]
                }
            },

            // --- 2. Pembacaan Detail Tipe & API ---
            {
                "name": "get_type_definition",
                "description": "Ambil definisi struktur data, signature fungsi dalam blok kode Rust, dan docstrings lengkap (Format: Markdown)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "full_path": { "type": "string", "description": "Contoh: rusqlite::Connection" }
                    },
                    "required": ["full_path"]
                }
            },
            {
                "name": "get_function_signature",
                "description": "Ambil signature persis dari fungsi/method (parameter input, generic bounds, dan return type) tanpa docstring panjang (Format: Markdown Code Block)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "full_path": { "type": "string", "description": "Contoh: rusqlite::Connection::open_with_flags" }
                    },
                    "required": ["full_path"]
                }
            },
            {
                "name": "get_associated_methods",
                "description": "Dapatkan daftar semua method/fungsi yang terhubung (inherent impls) dengan Struct atau Enum tertentu (Format: Markdown)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "struct_path": { "type": "string", "description": "Contoh: rusqlite::Connection" }
                    },
                    "required": ["struct_path"]
                }
            },
            {
                "name": "get_struct_fields",
                "description": "Dapatkan daftar seluruh field beserta tipe datanya untuk Struct tertentu atau variant dari Enum (Format: Markdown Table)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "struct_path": { "type": "string", "description": "Contoh: rusqlite::OpenFlags" }
                    },
                    "required": ["struct_path"]
                }
            },

            // --- 3. Relasi, Contoh Kode & Advanced Metadata ---
            {
                "name": "get_trait_impls",
                "description": "Dapatkan daftar Trait yang diimplementasikan oleh suatu tipe data (Struct/Enum) (Format: Markdown Code Blocks)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "struct_path": { "type": "string", "description": "Contoh: rusqlite::Connection" }
                    },
                    "required": ["struct_path"]
                }
            },
            {
                "name": "search_examples",
                "description": "Ambil contoh kode penggunaan (code snippets / doc-tests) untuk simbol atau path tertentu (Format: Markdown Code Blocks)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "full_path": { "type": "string", "description": "Contoh: rusqlite::Connection::transaction" }
                    },
                    "required": ["full_path"]
                }
            },
            {
                "name": "get_reexports",
                "description": "Cek apakah suatu item merupakan re-export (pub use) dan dapatkan path aslinya (Format: Markdown List)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "full_path": { "type": "string", "description": "Contoh: rusqlite::Error" }
                    },
                    "required": ["full_path"]
                }
            }
        ]
    })
}
