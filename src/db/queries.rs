//! Modul query database SQLite untuk penyediaan data simbol dokumentasi Rust.
//!
//! Modul ini menyediakan implementasi metode pencarian, ekstraksi tipe data,
//! navigasi modul, hingga pengambil sampel kode yang langsung diformat ke Markdown (`.md`)
//! untuk dikonsumsi oleh AI Agent via MCP Server.

use super::DbManager;
use anyhow::Result;
use rusqlite::params;

impl DbManager {
    /// Mengambil daftar semua crate beserta versinya yang sudah ter-index dalam bentuk List Markdown.
    ///
    /// # Returns
    /// Mengembalikan `Result<String>` berisi daftar crate dan versi berformat Markdown list.
    ///
    /// # Errors
    /// Mengembalikan error jika eksekusi SQL prepared statement gagal.
    pub fn get_crate_list(&self) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT crate_name, crate_version FROM items ORDER BY crate_name",
        )?;

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
            return Ok("Belum ada crate yang ter-index di database.".to_string());
        }

        Ok(md)
    }

    /// Melakukan pencarian simbol berbasis FTS5 yang disajikan dalam bentuk Tabel Markdown.
    ///
    /// # Arguments
    /// * `query` - Kata kunci nama atau path simbol yang dicari.
    /// * `kind_filter` - Filter tipe item opsional (misal: `"struct"`, `"function"`, `"trait"`, `"enum"`).
    ///
    /// # Returns
    /// Mengembalikan `Result<String>` berupa tabel Markdown yang memuat hasil pencarian.
    ///
    /// # Errors
    /// Mengembalikan error jika pencarian FTS5 gagal dieksekusi.
    pub fn search(&self, query: &str, kind_filter: Option<&str>) -> Result<String> {
        let clean_query = query
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
            .collect::<String>();

        if clean_query.is_empty() {
            return Ok("Query pencarian kosong.".to_string());
        }

        let formatted_query = format!("{}*", clean_query);

        let mut sql = String::from(
            "SELECT crate_name, kind, name, full_path, docs FROM items
             WHERE rowid IN (SELECT rowid FROM items_fts WHERE items_fts MATCH ?1 ORDER BY rank)",
        );

        if let Some(kind) = kind_filter {
            sql.push_str(&format!(" AND kind = '{}'", kind));
        }

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
            return Ok(format!("Tidak ditemukan simbol yang cocok dengan query `{}`.", query));
        }

        Ok(md)
    }

    /// Mengambil daftar isi item dalam suatu modul dalam bentuk Tabel Markdown.
    ///
    /// # Arguments
    /// * `module_path` - Canonical path dari modul induk (contoh: `"rusqlite::types"`).
    ///
    /// # Returns
    /// Mengembalikan `Result<String>` berisi daftar item anak dalam bentuk tabel Markdown.
    ///
    /// # Errors
    /// Mengembalikan error jika prepared statement SQL mengalami kegagalan.
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
            return Ok(format!("Modul `{}` kosong atau tidak ditemukan.", module_path));
        }

        Ok(md)
    }

    /// Mengambil definisi tipe lengkap, signature (blok kode Rust), dan dokumentasi penuh.
    ///
    /// # Arguments
    /// * `full_path` - Path lengkap ke simbol (contoh: `"rusqlite::Connection"`).
    ///
    /// # Returns
    /// Mengembalikan `Result<String>` berupa teks Markdown berisi metadata, signature kode, dan docstring.
    ///
    /// # Errors
    /// Mengembalikan error jika pembacaan dari database gagal.
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
            Ok(format!("Simbol `{}` tidak ditemukan.", full_path))
        }
    }

    /// Mengambil ringkasan signature fungsi/method dalam blok kode Rust.
    ///
    /// # Arguments
    /// * `full_path` - Path lengkap ke fungsi/method (contoh: `"rusqlite::Connection::open"`).
    ///
    /// # Returns
    /// Mengembalikan `Result<String>` berisi signature fungsi dalam blok kode Markdown.
    ///
    /// # Errors
    /// Mengembalikan error jika terjadi kegagalan query SQL.
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
            Ok(format!("Fungsi `{}` tidak ditemukan.", full_path))
        }
    }

    /// Mengambil daftar method/fungsi terhubung milik sebuah Struct/Enum.
    ///
    /// # Arguments
    /// * `struct_path` - Path lengkap dari Struct atau Enum (contoh: `"rusqlite::Connection"`).
    ///
    /// # Returns
    /// Mengembalikan `Result<String>` berisi daftar method berformat Markdown.
    ///
    /// # Errors
    /// Mengembalikan error jika pembacaan tabel SQLite bermasalah.
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
            return Ok(format!("Tidak ditemukan associated methods untuk `{}`.", struct_path));
        }

        Ok(md)
    }

    /// Mengambil daftar field internal Struct atau variant dari Enum.
    ///
    /// # Arguments
    /// * `struct_path` - Path lengkap Struct atau Enum (contoh: `"rusqlite::OpenFlags"`).
    ///
    /// # Returns
    /// Mengembalikan `Result<String>` berisi tabel Markdown dari field atau variant.
    ///
    /// # Errors
    /// Mengembalikan error jika eksekusi SQL gagal.
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
                r.0, r.1, r.3, r.4.replace('\n', " ")
            ));
            count += 1;
        }

        if count == 0 {
            return Ok(format!("Tidak ditemukan field/variant untuk `{}`.", struct_path));
        }

        Ok(md)
    }

    /// Mengambil daftar Trait yang diimplementasikan oleh sebuah Struct/Enum.
    ///
    /// # Arguments
    /// * `struct_path` - Path lengkap Struct atau Enum yang dicari implementasi trait-nya.
    ///
    /// # Returns
    /// Mengembalikan `Result<String>` berisi daftar blok `impl` berformat Markdown.
    ///
    /// # Errors
    /// Mengembalikan error jika query wildcard SQLite gagal.
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
            return Ok(format!("Tidak ditemukan implementasi trait untuk `{}`.", struct_path));
        }

        Ok(md)
    }

    /// Mengekstrak seluruh blok contoh kode (` ```rust ... ``` `) dari dokumentasi simbol.
    ///
    /// # Arguments
    /// * `full_path` - Path lengkap simbol yang dicari contoh kodenya.
    ///
    /// # Returns
    /// Mengembalikan `Result<String>` berisi potongan contoh kode yang diformat dalam Markdown.
    ///
    /// # Errors
    /// Mengembalikan error jika prepared statement atau query ke tabel `items` mengalami kegagalan.
    pub fn search_examples(&self, full_path: &str) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "SELECT docs FROM items WHERE full_path = ?1",
        )?;

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
            return Ok(format!("Tidak ditemukan contoh kode untuk `{}`.", full_path));
        }

        Ok(md)
    }

    /// Memeriksa apakah suatu simbol merupakan alias tipe data atau *re-export* (`pub use`).
    ///
    /// # Arguments
    /// * `full_path` - Path alias yang ingin diperiksa target aslinya.
    ///
    /// # Returns
    /// Mengembalikan `Result<String>` berisi daftar alias dan tipe aslinya berformat Markdown.
    ///
    /// # Errors
    /// Mengembalikan error jika query SQLite bermasalah.
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
            return Ok(format!("Tidak ditemukan alias/re-export untuk `{}`.", full_path));
        }

        Ok(md)
    }
}
