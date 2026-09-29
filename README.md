# Rustdoc MCP Server

`Rustdoc MCP Server` is a high-performance Model Context Protocol (MCP) server built in Rust. It enables AI Assistants (such as Claude Desktop, Cursor, Zed, and Windsurf) to instantly explore, search, and query local Rust documentation artifacts stored in a local SQLite database with FTS5 indexing.

---

## Features

- **SQLite Database Integration**: Utilizes SQLite with Write-Ahead Logging (WAL) mode and fine-tuned PRAGMA settings for ultra-fast, concurrent queries (<10ms response time).
- **FTS5 Full-Text Search**: Fast, ranked retrieval of Rust symbols, signatures, and docstrings.
- **Automated Rustdoc Generation**: Automatically detects missing documentation and executes `cargo doc` in the background—no manual build steps required.
- **Dynamic Crate Recognition**: Automatically parses `Cargo.toml` to extract the local crate name and direct dependencies without hardcoded paths.
- **Incremental Indexing**: Efficiently updates documentation on a per-crate basis when dependencies or code change.
- **JSON-RPC 2.0 via STDIO**: Fully compliant with the MCP specification over standard input/output.
- **LLM-Optimized Output**: Formats all tool query results into rich Markdown tables, code blocks, and lists ready for LLM consumption.

---

## How It Works

### 1. Zero-Config Initialization

When launched, `rustdoc-mcp-server` initializes a read-only SQLite connection for immediate client handshakes (`<10ms`). In parallel, it triggers a non-blocking background task to:

1. Parse the local project's `Cargo.toml` for crate names and direct dependencies.
2. Check `target/doc` for generated JSON artifacts. If missing, it automatically runs `cargo +nightly doc --no-deps -Zunstable-options --output-format json`.
3. Index new or updated symbols into the SQLite FTS5 database.

### 2. MCP JSON-RPC Communication

Communicating via `stdin` and `stdout`, the server handles standard MCP endpoints:

- `initialize`: Performs capabilities handshake.
- `tools/list`: Returns schemas for available documentation tools.
- `tools/call`: Executes SQL queries and returns formatted Markdown responses.

---

## MCP Tools Reference

The server registers 10 specialized tools categorized into three groups:

### 1. Discovery & Exploration

- **`get_crate_list`**: Lists all currently indexed crates and their versions.
- **`search_symbols`**: Full-text search across symbols with optional `kind` filtering (e.g., `struct`, `function`, `trait`, `enum`).
- **`get_module_contents`**: Lists all items inside a specific module path (e.g., `rusqlite::types`).

### 2. Type & API Inspection

- **`get_type_definition`**: Retrieves full struct/enum definitions, signatures, and docstrings.
- **`get_function_signature`**: Fetches exact function/method signatures (parameters, generic bounds, return types).
- **`get_associated_methods`**: Lists all inherent methods (`impl` blocks) attached to a struct/enum.
- **`get_struct_fields`**: Returns table of fields or enum variants for a given type.

### 3. Relations & Examples

- **`get_trait_impls`**: Lists traits implemented by a type.
- **`search_examples`**: Extracts code blocks/doc-tests from symbol documentation.
- **`get_reexports`**: Resolves type aliases (`pub use`) to their original target paths.

---

## Client Integration Guide

### Zed Editor

Add the server under `context_servers` in your `settings.json`:

```json
{
  "context_servers": {
    "rustdoc-mcp": {
      "enabled": true,
      "command": "/path/to/rustdoc-mcp-server/target/release/rustdoc-mcp-server",
      "args": []
    }
  }
}
```

### Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "rustdoc": {
      "command": "/path/to/rustdoc-mcp-server/target/release/rustdoc-mcp-server",
      "cwd": "/path/to/your/rust/project"
    }
  }
}
```

### Cursor / Windsurf

Configure an MCP server entry using STDIO transport pointing to the compiled binary location.

---

## Getting Started

### Prerequisites

- **Rust Toolchain**: (Edition 2021)
- **Nightly Toolchain** (Optional, for auto-generating rustdoc JSON):

```bash
$ rustup toolchain install nightly

```

### Build from Source

```bash
# Clone repository
$ git clone [https://github.com/your-repo/rustdoc-mcp-server](https://github.com/your-repo/rustdoc-mcp-server)
$ cd rustdoc-mcp-server

# Build optimized binary
$ cargo build --release

# Run binary (or reference in your editor's MCP settings)
$ ./target/release/rustdoc-mcp-server

```

---

## Database Schema

The SQLite database (`target/rustdoc_mcp.sqlite`) utilizes two primary tables:

- **`items`**: Stores raw symbol metadata (`id`, `crate_name`, `crate_version`, `kind`, `name`, `full_path`, `docs`, `signature`, `parent_path`).
- **`items_fts`**: FTS5 virtual table indexing `name`, `full_path`, and `docs` with automated sync triggers.

---

## Contribution

Contributions to `rustdoc-mcp-server` are welcome! To contribute:

1. Fork the repository and create your feature branch.
2. Ensure your changes pass formatting and linting (`cargo fmt`, `cargo clippy`).
3. Open a Pull Request with a clear description of changes.

---

## License

This project is licensed under the [MIT License](https://www.google.com/search?q=LICENSE).
