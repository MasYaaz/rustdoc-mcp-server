# rustdoc-mcp-server

`rustdoc-mcp-server` is a high-performance server application designed to support the Model Context Protocol (MCP) via JSON-RPC 2.0. It provides a platform for exploring and querying Rust documentation artifacts stored in a local SQLite database with FTS5 indexing.

---

## Features

- **SQLite Database Integration**: Utilizes SQLite with Write-Ahead Logging (WAL) mode for concurrent high-performance queries.
- **FTS5 Full-Text Search**: Enables fast and flexible retrieval of Rust documentation metadata.
- **JSON-RPC Support**: Implements asynchronous request/response handling over JSON-RPC 2.0.
- **Background Indexing**: Parses and indexes Rust documentation artifacts in the background for up-to-date content.
- **Modular Design**: Organized into database management, MCP request handling, and tool execution modules.

---

## Components Overview

### Main Modules

#### 1. **[`db` Module](src/db)**
- Responsible for SQLite database initialization and management.
- Implements schema creation, database optimizations, and triggers.
- Handles Full-Text Search queries through the `items_fts` virtual table.

#### 2. **[`mcp` Module](src/mcp)**
- Coordinates JSON-RPC communication between clients and the server.
- Maps incoming RPC commands to the appropriate tools and database queries.
- Provides structured JSON responses for a variety of client operations.

---

## How It Works

### 1. Initialization
The server initializes an `DbManager` with SQLite connection pooling and starts background indexing tasks. These tasks ensure that the documentation database is regularly synchronized with the target Rust project.

### 2. JSON-RPC Communication
The server reads input requests over `stdin` and responds to `stdout` in real-time. Supported JSON-RPC operations include:
- `initialize`: Establish capability handshake with the client.
- `tools/list`: Return the full list of available tools.
- `tools/call`: Execute specific database queries and return formatted Markdown results.

### 3. Tools & Queries
MCP tools allow flexible, specialized queries such as:
- Listing crates and modules
- Searching documentation symbols
- Retrieving function signatures, struct fields, or trait implementations
- Extracting code examples and relationships

---

## Getting Started

### Prerequisites
Ensure the following dependencies are installed:
- **Rust Toolchain**: (Edition 2021)
- **Cargo**: for building and managing Rust packages

### Build and Run
```bash
# Clone repository
$ git clone https://github.com/your-repo/rustdoc-mcp-server
$ cd rustdoc-mcp-server

# Build the project
$ cargo build --release

# Run the server
$ ./target/release/rustdoc-mcp-server
```
---

## Database Schema
The database contains two primary tables designed for fast symbol lookups:
- **`items`**: Stores metadata about Rust symbols (e.g., structs, functions, enums).
- **`items_fts`**: FTS5 table for full-text queries.

---

## Contribution
Contributions to `rustdoc-mcp-server` are welcome! To contribute, follow these steps:
1. Fork the repository and create your branch.
2. Commit your changes with descriptive messages.
3. Open a Pull Request.

---

## License
This project is licensed under the MIT License.
