//! Registration and schema provider module for Model Context Protocol (MCP) Tools.
//!
//! This module defines the list of all MCP tools supported by the server,
//! complete with tool names, Markdown-formatted descriptions, and JSON Schema definitions
//! for input arguments (`inputSchema`).

use serde_json::json;

/// Returns a JSON object containing schemas and descriptions for all available MCP tools.
///
/// All tools return Markdown-formatted (`.md`) output optimized for LLM / AI Agent consumption.
///
/// Registered tools are categorized into three primary groups:
/// 1. **Discovery & Exploration**: `get_crate_list`, `search_symbols`, `get_module_contents`
/// 2. **Type & API Inspection**: `get_type_definition`, `get_function_signature`, `get_associated_methods`, `get_struct_fields`
/// 3. **Relations, Code Examples & Metadata**: `get_trait_impls`, `search_examples`, `get_reexports`
///
/// # Returns
/// Returns a `serde_json::Value` representing the MCP specification JSON response for the `tools/list` method.
pub fn get_tools_list() -> serde_json::Value {
    json!({
        "tools": [
            // --- 1. Discovery & Exploration ---
            {
                "name": "get_crate_list",
                "description": "Get a list of all currently indexed crates and their versions in the database (Format: Markdown List)",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "search_symbols",
                "description": "Search for functions, structs, enums, or modules in Rust documentation using FTS5 with an optional 'kind' filter (Format: Markdown Table)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Symbol name or search keyword" },
                        "kind": { "type": "string", "description": "Optional symbol type filter (e.g., 'struct', 'function', 'trait', 'enum')" }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "get_module_contents",
                "description": "Get child items (structs, functions, sub-modules) contained inside a specific module path (Format: Markdown Table)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "module_path": { "type": "string", "description": "Canonical module path (e.g., rusqlite::types)" }
                    },
                    "required": ["module_path"]
                }
            },

            // --- 2. Type & API Inspection ---
            {
                "name": "get_type_definition",
                "description": "Get full data structure definitions, code block signatures, and docstrings for a symbol (Format: Markdown)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "full_path": { "type": "string", "description": "Canonical path (e.g., rusqlite::Connection)" }
                    },
                    "required": ["full_path"]
                }
            },
            {
                "name": "get_function_signature",
                "description": "Get exact function or method signature (parameters, generic bounds, and return type) without long docstrings (Format: Markdown Code Block)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "full_path": { "type": "string", "description": "Canonical path (e.g., rusqlite::Connection::open_with_flags)" }
                    },
                    "required": ["full_path"]
                }
            },
            {
                "name": "get_associated_methods",
                "description": "Get a list of all associated methods or functions (inherent impls) attached to a specific Struct or Enum (Format: Markdown)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "struct_path": { "type": "string", "description": "Canonical struct/enum path (e.g., rusqlite::Connection)" }
                    },
                    "required": ["struct_path"]
                }
            },
            {
                "name": "get_struct_fields",
                "description": "Get fields and types for a Struct or variants for an Enum (Format: Markdown Table)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "struct_path": { "type": "string", "description": "Canonical struct/enum path (e.g., rusqlite::OpenFlags)" }
                    },
                    "required": ["struct_path"]
                }
            },

            // --- 3. Relations, Code Examples & Metadata ---
            {
                "name": "get_trait_impls",
                "description": "Get a list of traits implemented by a given type (Struct or Enum) (Format: Markdown Code Blocks)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "struct_path": { "type": "string", "description": "Canonical type path (e.g., rusqlite::Connection)" }
                    },
                    "required": ["struct_path"]
                }
            },
            {
                "name": "search_examples",
                "description": "Extract usage code snippets or doc-tests embedded within documentation for a symbol (Format: Markdown Code Blocks)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "full_path": { "type": "string", "description": "Canonical symbol path (e.g., rusqlite::Connection::transaction)" }
                    },
                    "required": ["full_path"]
                }
            },
            {
                "name": "get_reexports",
                "description": "Check if a symbol is a re-export (pub use / type alias) and resolve its original target path (Format: Markdown List)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "full_path": { "type": "string", "description": "Canonical alias path (e.g., rusqlite::Error)" }
                    },
                    "required": ["full_path"]
                }
            }
        ]
    })
}
