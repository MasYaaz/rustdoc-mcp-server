//! Model Context Protocol (MCP) and JSON-RPC protocol handling module.
//!
//! This module manages JSON-RPC 2.0 communication between MCP Clients (such as Claude / AI Agents)
//! and the server, mapping `tools/call` executions to SQLite database query methods
//! that return Markdown (`.md`) formatted output.

pub mod tools;

use crate::db::DbManager;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// DTO representing an incoming JSON-RPC 2.0 Request from the client.
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct JsonRpcRequest {
    /// JSON-RPC protocol version (typically `"2.0"`).
    pub jsonrpc: String,
    /// Unique request ID supplied by the client (can be number, string, or null).
    pub id: Option<Value>,
    /// JSON-RPC method name invoked (e.g., `"initialize"`, `"tools/list"`, `"tools/call"`).
    pub method: String,
    /// Additional payload parameters attached to the request.
    pub params: Option<Value>,
}

/// DTO representing an outgoing JSON-RPC 2.0 Response to the client.
#[derive(Serialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC protocol version (`"2.0"`).
    pub jsonrpc: String,
    /// Request ID matching the corresponding `JsonRpcRequest`.
    pub id: Option<Value>,
    /// Execution result object upon successful request handling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error object if JSON-RPC processing fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

/// Processes incoming JSON-RPC requests and returns a structured `JsonRpcResponse`.
///
/// Handles three core endpoints per the MCP specification:
/// - `"initialize"`: Performs initial server handshake and capability reporting.
/// - `"tools/list"`: Returns schemas for all available MCP tools defined in the `tools` module.
/// - `"tools/call"`: Executes `DbManager` Markdown query functions matching tool name and arguments.
///
/// # Arguments
/// * `db` - Reference to the [`DbManager`] instance for querying documentation data.
/// * `req` - [`JsonRpcRequest`] payload received from the client.
///
/// # Returns
/// Returns a [`JsonRpcResponse`] containing Markdown formatted content or JSON-RPC error objects.
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

            // Helper to format Result<String> queries into standard MCP content objects
            let format_response = |res: anyhow::Result<String>| match res {
                Ok(md) => json!({ "content": [{ "type": "text", "text": md }] }),
                Err(e) => {
                    json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                }
            };

            let content = match name {
                // --- 1. Discovery & Exploration ---
                "get_crate_list" => format_response(db.get_crate_list()),
                "search_symbols" => {
                    let query = args["query"].as_str().unwrap_or("");
                    let kind = args["kind"].as_str();
                    format_response(db.search(query, kind))
                }
                "get_module_contents" => {
                    let module_path = args["module_path"].as_str().unwrap_or("");
                    format_response(db.get_module_contents(module_path))
                }

                // --- 2. Type & API Inspection ---
                "get_type_definition" => {
                    let full_path = args["full_path"].as_str().unwrap_or("");
                    format_response(db.get_type_definition(full_path))
                }
                "get_function_signature" => {
                    let full_path = args["full_path"].as_str().unwrap_or("");
                    format_response(db.get_function_signature(full_path))
                }
                "get_associated_methods" => {
                    let struct_path = args["struct_path"].as_str().unwrap_or("");
                    format_response(db.get_associated_methods(struct_path))
                }
                "get_struct_fields" => {
                    let struct_path = args["struct_path"].as_str().unwrap_or("");
                    format_response(db.get_struct_fields(struct_path))
                }

                // --- 3. Relations, Code Examples & Metadata ---
                "get_trait_impls" => {
                    let struct_path = args["struct_path"].as_str().unwrap_or("");
                    format_response(db.get_trait_impls(struct_path))
                }
                "search_examples" => {
                    let full_path = args["full_path"].as_str().unwrap_or("");
                    format_response(db.search_examples(full_path))
                }
                "get_reexports" => {
                    let full_path = args["full_path"].as_str().unwrap_or("");
                    format_response(db.get_reexports(full_path))
                }

                _ => {
                    json!({ "content": [{ "type": "text", "text": "Unknown tool" }], "isError": true })
                }
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
