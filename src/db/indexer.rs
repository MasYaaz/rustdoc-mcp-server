//! Modul pemrosesan dan peng-indeks-an AST rustdoc JSON ke SQLite.
//!
//! Modul ini membaca artefak dokumentasi JSON yang dihasilkan oleh `cargo doc`,
//! memetakan simbol-simbol di dalamnya ke skema SQLite, serta memfilter ketergantungan
//! (*dependencies*) berdasarkan daftar yang ada di `Cargo.toml`.

use super::DbManager;
use anyhow::Result;
use cargo_toml::Manifest;
use rusqlite::params;
use rustdoc_types::{Crate, ItemEnum};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

impl DbManager {
    /// Membaca file rustdoc JSON dan memasukkan semua simbol yang terurai ke dalam database SQLite.
    ///
    /// Fungsi ini mengekstrak informasi tipe data, nama, path lengkap, versi crate,
    /// serta membuat ID unik berbasis `crate_name:version:id` sebelum menyimpannya via transaksi SQL.
    ///
    /// # Arguments
    /// * `json_path` - Path file `.json` rustdoc yang akan diproses.
    ///
    /// # Returns
    /// Mengembalikan `Ok(usize)` berupa jumlah item yang berhasil di-index ke dalam database.
    ///
    /// # Errors
    /// Mengembalikan error jika terjadi kegagalan pembacaan file atau kesalahan transaksi SQL.
    pub fn index_json_file(&mut self, json_path: &Path) -> Result<usize> {
        let content = fs::read_to_string(json_path)?;
        let raw_crate: Crate = match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(_) => return Ok(0),
        };

        let file_stem = json_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown_crate");

        let crate_real_name = raw_crate
            .index
            .get(&raw_crate.root)
            .and_then(|item| item.name.as_deref())
            .unwrap_or(file_stem);

        let tx = self.conn.transaction()?;
        let mut count = 0;

        let generic_trait_methods: HashSet<&str> = [
            "clone", "clone_into", "eq", "ne", "fmt", "deref", "deref_mut", "drop", "hash",
            "from", "into", "try_from", "try_into", "as_ref", "borrow", "borrow_mut", "to_owned",
        ]
        .into_iter()
        .collect();

        for (id, item) in &raw_crate.index {
            let name = match &item.name {
                Some(n) if !n.is_empty() => n.as_str(),
                _ => continue,
            };

            if name.parse::<usize>().is_ok() {
                continue;
            }

            let kind = match &item.inner {
                ItemEnum::Struct(_) => "struct",
                ItemEnum::Enum(_) => "enum",
                ItemEnum::Function(_) => "function",
                ItemEnum::Trait(_) => "trait",
                ItemEnum::Module(_) => "module",
                ItemEnum::TypeAlias(_) => "type_alias",
                ItemEnum::Constant { .. } => "constant",
                ItemEnum::Static(_) => "static",
                ItemEnum::Macro(_) => "macro",
                ItemEnum::ProcMacro(_) => "proc_macro",
                ItemEnum::StructField(_) => continue,
                ItemEnum::AssocType { .. } => "assoc_type",
                ItemEnum::AssocConst { .. } => "assoc_const",
                ItemEnum::Variant(_) => "enum_variant",
                ItemEnum::Impl(_) => "impl",
                _ => "other",
            };

            let is_in_paths = raw_crate.paths.contains_key(id);
            let docs = item.docs.as_deref().unwrap_or("");

            if !is_in_paths && docs.is_empty() && generic_trait_methods.contains(name) {
                continue;
            }

            let full_path = if let Some(path_summary) = raw_crate.paths.get(id) {
                path_summary.path.join("::")
            } else {
                format!("{}::{}", crate_real_name, name)
            };

            // Hitung Parent Path untuk fitur modul hierarki (Tree Module)
            let parent_path = if let Some((parent, _)) = full_path.rsplit_once("::") {
                parent.to_string()
            } else {
                crate_real_name.to_string()
            };

            let signature = format!("{:?}", item.inner);
            let crate_version = raw_crate.crate_version.as_deref().unwrap_or("0.0.0");
            let item_id = format!("{}:{}:{}", crate_real_name, crate_version, id.0);

            tx.execute(
                "INSERT OR REPLACE INTO items (id, crate_name, crate_version, kind, name, full_path, docs, signature, parent_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![item_id, crate_real_name, crate_version, kind, name, full_path, docs, signature, parent_path],
            )?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// Mengekstrak daftar nama *direct dependencies* langsung dari file `Cargo.toml`.
    ///
    /// Karakter `-` pada nama crate (misal: `rustdoc-types`) otomatis diubah menjadi `_` (`rustdoc_types`)
    /// agar sesuai dengan penamaan pustaka/file pada folder target doc.
    ///
    /// # Arguments
    /// * `manifest_path` - Path lokasi file `Cargo.toml`.
    ///
    /// # Returns
    /// Mengembalikan `HashSet<String>` yang berisi daftar nama crate ketergantungan utama.
    pub fn get_direct_dependencies(manifest_path: &Path) -> HashSet<String> {
        let mut set = HashSet::new();
        if let Ok(manifest) = Manifest::from_path(manifest_path) {
            for (dep_name, _) in manifest.dependencies {
                set.insert(dep_name.replace('-', "_"));
            }
        }
        set
    }

    /// Memindai direktori target doc dan meng-index file JSON yang relevan.
        ///
        /// Jika database SQLite sudah memiliki data (hasil indeks sebelumnya), proses pemindaian
        /// akan dilewati secara otomatis untuk mengoptimalkan kecepatan startup server (<10ms).
        ///
        /// Pemindaian difilter sehingga hanya file JSON yang terdaftar sebagai *direct dependency*
        /// di `Cargo.toml` atau crate lokal yang akan dimasukkan ke database SQLite.
        ///
        /// # Arguments
        /// * `target_dir` - Path direktori target doc (contoh: `"target/doc"`).
        /// * `manifest_path` - Path lokasi file `Cargo.toml`.
        ///
        /// # Errors
        /// Mengembalikan error jika terjadi kegagalan query SQLite atau pemrosesan direktori.
        pub fn scan_and_index_target(&mut self, target_dir: &Path, manifest_path: &Path) -> Result<()> {
            if !target_dir.exists() {
                return Ok(());
            }

            // Cek apakah DB sudah pernah terisi. Jika sudah ada data, skip scanning!
            let count: i64 = self
                .conn
                .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
                .unwrap_or(0);

            if count > 0 {
                return Ok(());
            }

            let direct_deps = Self::get_direct_dependencies(manifest_path);

            for entry in WalkDir::new(target_dir).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

                    // Filter: Hanya index jika file JSON termasuk direct dependency atau crate lokal sendiri
                    if direct_deps.contains(file_stem) || file_stem == "my_crate_name" {
                        let _ = self.index_json_file(path);
                    }
                }
            }
            Ok(())
        }
}
