//! SQLite query module for serving Rust documentation symbol data.
//!
//! This module provides methods for symbol searches, type definitions,
//! module navigation, and code example extraction directly formatted into
//! Markdown (`.md`) for consumption by AI Agents via the MCP Server.

use super::DbManager;
use anyhow::Result;
use rusqlite::params;

impl DbManager {
    /// Retrieves a list of all indexed crates and their versions as a Markdown list.
    ///
    /// # Returns
    /// Returns a `Result<String>` containing the list of crates and versions formatted as Markdown.
    ///
    /// # Errors
    /// Returns an error if SQL prepared statement execution fails.
    pub fn get_crate_list(&self) -> Result<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT crate_name, crate_version FROM items ORDER BY crate_name")?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut md = String::from("### Indexed Crates\n\n");
        let mut count = 0;

        for r in rows.flatten() {
            md.push_str(&format!("- **{}** (v{})\n", r.0, r.1));
            count += 1;
        }

        if count == 0 {
            return Ok("No indexed crates found in the database.".to_string());
        }

        Ok(md)
    }

    /// Performs FTS5 full-text symbol search and returns the results as a Markdown Table.
    ///
    /// Output is capped at 50 records to prevent excessive LLM token usage.
    ///
    /// # Arguments
    /// * `query` - Keyword matching symbol name or path.
    /// * `kind_filter` - Optional item type filter (e.g., `"struct"`, `"function"`, `"trait"`, `"enum"`).
    ///
    /// # Returns
    /// Returns a `Result<String>` containing a Markdown table with the search results.
    ///
    /// # Errors
    /// Returns an error if FTS5 query execution fails.
    pub fn search(&self, query: &str, kind_filter: Option<&str>) -> Result<String> {
        let clean_query = query
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
            .collect::<String>();

        if clean_query.is_empty() {
            return Ok("Search query is empty.".to_string());
        }

        let formatted_query = format!("{}*", clean_query);

        let mut sql = String::from(
            "SELECT crate_name, kind, name, full_path, docs FROM items
             WHERE rowid IN (SELECT rowid FROM items_fts WHERE items_fts MATCH ?1 ORDER BY rank)",
        );

        if let Some(kind) = kind_filter {
            sql.push_str(&format!(" AND kind = '{}'", kind));
        }

        // Limit results to 50 rows to optimize LLM context window token usage
        sql.push_str(" LIMIT 50");

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![formatted_query], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        });

        let mut md = String::from("| Crate | Kind | Name | Full Path | Docs Snippet |\n");
        md.push_str("|---|---|---|---|---|\n");
        let mut count = 0;

        if let Ok(mapped) = rows {
            for r in mapped.flatten() {
                let docs_snippet = r.4.lines().take(2).collect::<Vec<_>>().join(" ");
                md.push_str(&format!(
                    "| `{}` | `{}` | **{}** | `{}` | {} |\n",
                    r.0, r.1, r.2, r.3, docs_snippet
                ));
                count += 1;
            }
        }

        if count == 0 {
            return Ok(format!("No symbols found matching query `{}`.", query));
        }

        Ok(md)
    }

    /// Retrieves module items in Markdown Table format.
    ///
    /// # Arguments
    /// * `module_path` - Canonical path of the parent module (e.g., `"rusqlite::types"`).
    ///
    /// # Returns
    /// Returns a `Result<String>` listing child items as a Markdown table.
    ///
    /// # Errors
    /// Returns an error if SQL statement preparation fails.
    pub fn get_module_contents(&self, module_path: &str) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "SELECT kind, name, full_path, docs FROM items WHERE parent_path = ?1 ORDER BY kind, name",
        )?;

        let rows = stmt.query_map(params![module_path], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut md = format!("### Module Contents: `{}`\n\n", module_path);
        md.push_str("| Kind | Name | Full Path | Description |\n");
        md.push_str("|---|---|---|---|\n");
        let mut count = 0;

        for r in rows.flatten() {
            let doc_first_line = r.3.lines().next().unwrap_or("").to_string();
            md.push_str(&format!(
                "| `{}` | **{}** | `{}` | {} |\n",
                r.0, r.1, r.2, doc_first_line
            ));
            count += 1;
        }

        if count == 0 {
            return Ok(format!(
                "Module `{}` is empty or was not found.",
                module_path
            ));
        }

        Ok(md)
    }

    /// Retrieves full type definition, code signature block, and documentation string.
    ///
    /// # Arguments
    /// * `full_path` - Canonical path to the symbol (e.g., `"rusqlite::Connection"`).
    ///
    /// # Returns
    /// Returns a `Result<String>` containing Markdown text with symbol metadata, signature, and docstring.
    ///
    /// # Errors
    /// Returns an error if reading from the database fails.
    pub fn get_type_definition(&self, full_path: &str) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "SELECT crate_name, kind, name, full_path, docs, signature FROM items WHERE full_path = ?1",
        )?;

        let mut rows = stmt.query(params![full_path])?;
        if let Some(row) = rows.next()? {
            let crate_name: String = row.get(0)?;
            let kind: String = row.get(1)?;
            let name: String = row.get(2)?;
            let docs: String = row.get(4)?;
            let signature: String = row.get(5)?;

            let mut md = format!("# `{}`\n\n", name);
            md.push_str(&format!("- **Kind:** `{}`\n", kind));
            md.push_str(&format!("- **Crate:** `{}`\n", crate_name));
            md.push_str(&format!("- **Path:** `{}`\n\n", full_path));

            md.push_str("### Signature\n```rust\n");
            md.push_str(&signature);
            md.push_str("\n```\n\n");

            if !docs.trim().is_empty() {
                md.push_str("### Documentation\n");
                md.push_str(&docs);
                md.push('\n');
            }

            Ok(md)
        } else {
            Ok(format!("Symbol `{}` was not found.", full_path))
        }
    }

    /// Retrieves function or method signature formatted as a Rust code block.
    ///
    /// # Arguments
    /// * `full_path` - Canonical path to the function/method (e.g., `"rusqlite::Connection::open"`).
    ///
    /// # Returns
    /// Returns a `Result<String>` containing the function signature in a Markdown code block.
    ///
    /// # Errors
    /// Returns an error if SQL query execution fails.
    pub fn get_function_signature(&self, full_path: &str) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "SELECT full_path, signature FROM items WHERE full_path = ?1 AND kind = 'function'",
        )?;

        let mut rows = stmt.query(params![full_path])?;
        if let Some(row) = rows.next()? {
            let signature: String = row.get(1)?;

            let mut md = format!("### Function Signature: `{}`\n\n", full_path);
            md.push_str("```rust\n");
            md.push_str(&signature);
            md.push_str("\n```\n");

            Ok(md)
        } else {
            Ok(format!("Function `{}` was not found.", full_path))
        }
    }

    /// Retrieves associated methods and functions belonging to a Struct or Enum.
    ///
    /// # Arguments
    /// * `struct_path` - Canonical path of the Struct or Enum (e.g., `"rusqlite::Connection"`).
    ///
    /// # Returns
    /// Returns a `Result<String>` listing associated methods in Markdown format.
    ///
    /// # Errors
    /// Returns an error if querying SQLite fails.
    pub fn get_associated_methods(&self, struct_path: &str) -> Result<String> {
        let parent_pattern = format!("{}::%", struct_path);
        let mut stmt = self.conn.prepare(
            "SELECT name, full_path, signature, docs FROM items
             WHERE kind = 'function' AND full_path LIKE ?1
             ORDER BY name",
        )?;

        let rows = stmt.query_map(params![parent_pattern], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut md = format!("### Associated Methods for `{}`\n\n", struct_path);
        let mut count = 0;

        for r in rows.flatten() {
            let doc_first_line = r.3.lines().next().unwrap_or("").to_string();
            md.push_str(&format!("#### `{}`\n", r.0));
            md.push_str(&format!("- **Full Path:** `{}`\n", r.1));
            md.push_str("```rust\n");
            md.push_str(&r.2);
            md.push_str("\n```\n");
            if !doc_first_line.is_empty() {
                md.push_str(&format!("_{}_\n\n", doc_first_line));
            }
            count += 1;
        }

        if count == 0 {
            return Ok(format!(
                "No associated methods found for `{}`.",
                struct_path
            ));
        }

        Ok(md)
    }

    /// Retrieves internal fields of a Struct or variants of an Enum.
    ///
    /// # Arguments
    /// * `struct_path` - Canonical path of the Struct or Enum (e.g., `"rusqlite::OpenFlags"`).
    ///
    /// # Returns
    /// Returns a `Result<String>` containing a Markdown table of fields or variants.
    ///
    /// # Errors
    /// Returns an error if SQL execution fails.
    pub fn get_struct_fields(&self, struct_path: &str) -> Result<String> {
        let parent_pattern = format!("{}::%", struct_path);
        let mut stmt = self.conn.prepare(
            "SELECT name, kind, full_path, signature, docs FROM items
             WHERE kind IN ('struct_field', 'enum_variant') AND full_path LIKE ?1
             ORDER BY name",
        )?;

        let rows = stmt.query_map(params![parent_pattern], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut md = format!("### Fields / Variants for `{}`\n\n", struct_path);
        md.push_str("| Name | Kind | Type Signature | Description |\n");
        md.push_str("|---|---|---|---|\n");
        let mut count = 0;

        for r in rows.flatten() {
            md.push_str(&format!(
                "| **{}** | `{}` | `{}` | {} |\n",
                r.0,
                r.1,
                r.3,
                r.4.replace('\n', " ")
            ));
            count += 1;
        }

        if count == 0 {
            return Ok(format!(
                "No fields or variants found for `{}`.",
                struct_path
            ));
        }

        Ok(md)
    }

    /// Retrieves trait implementations attached to a Struct or Enum.
    ///
    /// # Arguments
    /// * `struct_path` - Canonical path of the Struct or Enum.
    ///
    /// # Returns
    /// Returns a `Result<String>` containing `impl` blocks formatted in Markdown.
    ///
    /// # Errors
    /// Returns an error if SQLite query pattern fails.
    pub fn get_trait_impls(&self, struct_path: &str) -> Result<String> {
        let search_pattern = format!("%impl%for%{}%", struct_path);
        let mut stmt = self.conn.prepare(
            "SELECT full_path, signature, docs FROM items WHERE kind = 'impl' AND (full_path LIKE ?1 OR signature LIKE ?1)",
        )?;

        let rows = stmt.query_map(params![search_pattern], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut md = format!("### Trait Implementations for `{}`\n\n", struct_path);
        let mut count = 0;

        for r in rows.flatten() {
            md.push_str("```rust\n");
            md.push_str(&r.1);
            md.push_str("\n```\n");
            if !r.2.trim().is_empty() {
                md.push_str(&format!("{}\n\n", r.2));
            }
            count += 1;
        }

        if count == 0 {
            return Ok(format!(
                "No trait implementations found for `{}`.",
                struct_path
            ));
        }

        Ok(md)
    }

    /// Extracts code example blocks (` ```rust ... ``` `) embedded within symbol documentation.
    ///
    /// # Arguments
    /// * `full_path` - Canonical path of the symbol.
    ///
    /// # Returns
    /// Returns a `Result<String>` containing extracted code snippets formatted in Markdown.
    ///
    /// # Errors
    /// Returns an error if database statement or query fails.
    pub fn search_examples(&self, full_path: &str) -> Result<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT docs FROM items WHERE full_path = ?1")?;

        let mut rows = stmt.query(params![full_path])?;
        let mut md = format!("### Code Examples for `{}`\n\n", full_path);
        let mut example_count = 0;

        if let Some(row) = rows.next()? {
            let docs: String = row.get(0)?;
            let mut in_code_block = false;
            let mut current_block = String::new();

            for line in docs.lines() {
                if line.trim_start().starts_with("```") {
                    if in_code_block {
                        in_code_block = false;
                        if !current_block.trim().is_empty() {
                            example_count += 1;
                            md.push_str(&format!("#### Example {}\n```rust\n", example_count));
                            md.push_str(current_block.trim());
                            md.push_str("\n```\n\n");
                            current_block.clear();
                        }
                    } else {
                        in_code_block = true;
                    }
                } else if in_code_block {
                    current_block.push_str(line);
                    current_block.push('\n');
                }
            }
        }

        if example_count == 0 {
            return Ok(format!("No code examples found for `{}`.", full_path));
        }

        Ok(md)
    }

    /// Resolves type aliases or re-exported symbols (`pub use`).
    ///
    /// # Arguments
    /// * `full_path` - Path of the alias to inspect.
    ///
    /// # Returns
    /// Returns a `Result<String>` containing alias and target mapping in Markdown format.
    ///
    /// # Errors
    /// Returns an error if SQLite query execution encounters issues.
    pub fn get_reexports(&self, full_path: &str) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "SELECT name, full_path, signature FROM items WHERE kind = 'type_alias' AND full_path = ?1",
        )?;

        let rows = stmt.query_map(params![full_path], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut md = format!("### Re-exports / Type Aliases for `{}`\n\n", full_path);
        let mut count = 0;

        for r in rows.flatten() {
            md.push_str(&format!("- **Alias:** `{}`\n", r.0));
            md.push_str(&format!("- **Full Path:** `{}`\n", r.1));
            md.push_str(&format!("- **Target:** `{}`\n\n", r.2));
            count += 1;
        }

        if count == 0 {
            return Ok(format!(
                "No type aliases or re-exports found for `{}`.",
                full_path
            ));
        }

        Ok(md)
    }
}
