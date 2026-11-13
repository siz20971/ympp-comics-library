use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use rusqlite::{params, Connection, Result};
use crate::models::{BreadcrumbItem, ComicDetail, ComicFileItem, FolderContent, FolderItem, Library, SearchQuery, TagItem};

pub struct Database {
    #[allow(dead_code)]
    db_path: PathBuf,
    conn_pool: Mutex<Connection>,
}

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&db_path)?;
        let _ = conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA cache_size = -20000;"
        );
        let db = Database {
            db_path,
            conn_pool: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn conn(&self) -> Result<MutexGuard<'_, Connection>> {
        Ok(self.conn_pool.lock().unwrap())
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        if let Err(e) = conn.execute_batch("PRAGMA journal_mode = WAL;") {
            eprintln!("Warning: Failed to set WAL mode (possibly network share): {}", e);
            let _ = conn.execute_batch("PRAGMA journal_mode = DELETE;");
        }

        let user_version: i64 = conn.query_row("PRAGMA user_version;", [], |r| r.get(0)).unwrap_or(0);
        if user_version >= 2 {
            return Ok(());
        }

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS libraries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                root_path TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_scanned_at TEXT
            );

            CREATE TABLE IF NOT EXISTS folders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                library_id INTEGER NOT NULL,
                relative_path TEXT NOT NULL,
                name TEXT NOT NULL,
                parent_id INTEGER,
                rating INTEGER NOT NULL DEFAULT 0,
                is_new INTEGER NOT NULL DEFAULT 1,
                cover_status TEXT NOT NULL DEFAULT 'unknown',
                last_scanned_at TEXT,
                UNIQUE(library_id, relative_path),
                FOREIGN KEY(library_id) REFERENCES libraries(id) ON DELETE CASCADE,
                FOREIGN KEY(parent_id) REFERENCES folders(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS comic_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                folder_id INTEGER NOT NULL,
                relative_path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                extension TEXT NOT NULL,
                size INTEGER NOT NULL DEFAULT 0,
                modified_at TEXT NOT NULL,
                rating INTEGER NOT NULL DEFAULT 0,
                comment TEXT NOT NULL DEFAULT '',
                is_new INTEGER NOT NULL DEFAULT 1,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                FOREIGN KEY(folder_id) REFERENCES folders(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                is_system INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS folder_tags (
                folder_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (folder_id, tag_id),
                FOREIGN KEY(folder_id) REFERENCES folders(id) ON DELETE CASCADE,
                FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS comic_file_tags (
                comic_file_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (comic_file_id, tag_id),
                FOREIGN KEY(comic_file_id) REFERENCES comic_files(id) ON DELETE CASCADE,
                FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS cover_caches (
                comic_file_id INTEGER PRIMARY KEY,
                cache_path TEXT NOT NULL,
                file_modified_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'ready',
                error TEXT,
                FOREIGN KEY(comic_file_id) REFERENCES comic_files(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_folders_parent ON folders(parent_id);
            CREATE INDEX IF NOT EXISTS idx_folders_lib_parent ON folders(library_id, parent_id);
            CREATE INDEX IF NOT EXISTS idx_comic_files_folder ON comic_files(folder_id);
            CREATE INDEX IF NOT EXISTS idx_comic_files_rating ON comic_files(rating);
            CREATE INDEX IF NOT EXISTS idx_comic_files_is_new ON comic_files(is_new);
            CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);
            "#
        )?;

        // Auto-migrate existing DBs without failing on duplicate columns
        let mut existing_cols = std::collections::HashSet::new();
        {
            let mut info_stmt = conn.prepare("PRAGMA table_info(folders)")?;
            let cols = info_stmt.query_map([], |r| r.get::<_, String>(1))?;
            for col in cols.flatten() {
                existing_cols.insert(col);
            }
        }
        if !existing_cols.contains("is_new") {
            conn.execute("ALTER TABLE folders ADD COLUMN is_new INTEGER NOT NULL DEFAULT 1", [])?;
        }
        if !existing_cols.contains("cover_status") {
            conn.execute("ALTER TABLE folders ADD COLUMN cover_status TEXT NOT NULL DEFAULT 'unknown'", [])?;
        }

        conn.execute("CREATE INDEX IF NOT EXISTS idx_folders_is_new ON folders(is_new)", [])?;
        conn.execute_batch("PRAGMA user_version = 2;")?;

        Ok(())
    }

    // --- Library ---
    pub fn get_or_create_library(&self, root_path: &str) -> Result<Library> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();

        let mut stmt = conn.prepare("SELECT id, root_path, created_at, updated_at, last_scanned_at FROM libraries WHERE root_path = ?1")?;
        let existing = stmt.query_row(params![root_path], |row| {
            Ok(Library {
                id: row.get(0)?,
                root_path: row.get(1)?,
                display_path: None,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                last_scanned_at: row.get(4)?,
            })
        });

        match existing {
            Ok(lib) => Ok(lib),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                conn.execute(
                    "INSERT INTO libraries (root_path, created_at, updated_at) VALUES (?1, ?2, ?3)",
                    params![root_path, now, now],
                )?;
                let id = conn.last_insert_rowid();
                Ok(Library {
                    id,
                    root_path: root_path.to_string(),
                    display_path: None,
                    created_at: now.clone(),
                    updated_at: now,
                    last_scanned_at: None,
                })
            }
            Err(e) => Err(e),
        }
    }

    pub fn get_latest_library(&self) -> Result<Option<Library>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT id, root_path, created_at, updated_at, last_scanned_at FROM libraries ORDER BY id DESC LIMIT 1")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Library {
                id: row.get(0)?,
                root_path: row.get(1)?,
                display_path: None,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                last_scanned_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_library_scan_time(&self, lib_id: i64) -> Result<()> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE libraries SET last_scanned_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![now, now, lib_id],
        )?;
        Ok(())
    }

    // --- Folders ---
    pub fn upsert_folder(&self, lib_id: i64, rel_path: &str, name: &str, parent_id: Option<i64>) -> Result<i64> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO folders (library_id, relative_path, name, parent_id, last_scanned_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(library_id, relative_path) DO UPDATE SET
                name = excluded.name,
                parent_id = excluded.parent_id,
                last_scanned_at = excluded.last_scanned_at
            "#,
            params![lib_id, rel_path, name, parent_id, now],
        )?;

        let mut stmt = conn.prepare("SELECT id FROM folders WHERE library_id = ?1 AND relative_path = ?2")?;
        let id: i64 = stmt.query_row(params![lib_id, rel_path], |row| row.get(0))?;
        Ok(id)
    }

    pub fn get_folder_by_id(&self, folder_id: i64) -> Result<Option<FolderItem>> {
        let conn = self.conn()?;
        self.get_folder_by_id_with_conn(&conn, folder_id)
    }

    fn get_folder_by_id_with_conn(&self, conn: &Connection, folder_id: i64) -> Result<Option<FolderItem>> {
        let mut stmt = conn.prepare(
            "SELECT id, library_id, relative_path, name, parent_id, rating, is_new, cover_status FROM folders WHERE id = ?1"
        )?;
        let mut rows = stmt.query(params![folder_id])?;
        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let tags = self.get_direct_folder_tags(conn, id)?;
            Ok(Some(FolderItem {
                id,
                library_id: row.get(1)?,
                relative_path: row.get(2)?,
                name: row.get(3)?,
                parent_id: row.get(4)?,
                rating: row.get(5)?,
                is_new: row.get(6)?,
                cover_status: row.get(7)?,
                cover_data_url: None,
                direct_tags: tags,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_subfolders(&self, lib_id: i64, parent_id: Option<i64>) -> Result<Vec<FolderItem>> {
        let conn = self.conn()?;
        self.get_subfolders_with_conn(&conn, lib_id, parent_id)
    }

    fn get_subfolders_with_conn(&self, conn: &Connection, lib_id: i64, parent_id: Option<i64>) -> Result<Vec<FolderItem>> {
        let mut results = Vec::new();

        let row_tuples: Vec<(i64, i64, String, String, Option<i64>, i32, bool, String)> = match parent_id {
            Some(pid) => {
                let mut stmt = conn.prepare(
                    "SELECT id, library_id, relative_path, name, parent_id, rating, is_new, cover_status FROM folders WHERE library_id = ?1 AND parent_id = ?2 ORDER BY name ASC"
                )?;
                let rows = stmt.query_map(params![lib_id, pid], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?))
                })?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id, library_id, relative_path, name, parent_id, rating, is_new, cover_status FROM folders WHERE library_id = ?1 AND (parent_id IS NULL OR parent_id = 0) ORDER BY name ASC"
                )?;
                let rows = stmt.query_map(params![lib_id], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?))
                })?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
        };

        if row_tuples.is_empty() {
            return Ok(results);
        }

        // Batch fetch all direct tags for these subfolders in a single query
        let mut tag_map: HashMap<i64, Vec<TagItem>> = HashMap::new();
        match parent_id {
            Some(pid) => {
                let mut tag_stmt = conn.prepare(
                    r#"
                    SELECT ft.folder_id, t.id, t.name, t.display_name, t.is_system
                    FROM folder_tags ft
                    JOIN tags t ON ft.tag_id = t.id
                    JOIN folders f ON ft.folder_id = f.id
                    WHERE f.library_id = ?1 AND f.parent_id = ?2
                    ORDER BY t.display_name ASC
                    "#
                )?;
                let tag_rows = tag_stmt.query_map(params![lib_id, pid], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        TagItem {
                            id: r.get(1)?,
                            name: r.get(2)?,
                            display_name: r.get(3)?,
                            is_system: r.get(4)?,
                        },
                    ))
                })?;
                for tr in tag_rows {
                    let (fid, tag) = tr?;
                    tag_map.entry(fid).or_default().push(tag);
                }
            }
            None => {
                let mut tag_stmt = conn.prepare(
                    r#"
                    SELECT ft.folder_id, t.id, t.name, t.display_name, t.is_system
                    FROM folder_tags ft
                    JOIN tags t ON ft.tag_id = t.id
                    JOIN folders f ON ft.folder_id = f.id
                    WHERE f.library_id = ?1 AND (f.parent_id IS NULL OR f.parent_id = 0)
                    ORDER BY t.display_name ASC
                    "#
                )?;
                let tag_rows = tag_stmt.query_map(params![lib_id], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        TagItem {
                            id: r.get(1)?,
                            name: r.get(2)?,
                            display_name: r.get(3)?,
                            is_system: r.get(4)?,
                        },
                    ))
                })?;
                for tr in tag_rows {
                    let (fid, tag) = tr?;
                    tag_map.entry(fid).or_default().push(tag);
                }
            }
        }

        for (id, lib_id, rel_path, name, pid, rating, is_new, cover_status) in row_tuples {
            let tags = tag_map.remove(&id).unwrap_or_default();
            results.push(FolderItem {
                id,
                library_id: lib_id,
                relative_path: rel_path,
                name,
                parent_id: pid,
                rating,
                is_new,
                cover_status,
                cover_data_url: None,
                direct_tags: tags,
            });
        }
        Ok(results)
    }

    /// Fetches all data needed for `get_folder_content` under a single DB lock.
    pub fn get_folder_content_bundle(&self, folder_id: Option<i64>) -> Result<Option<FolderContent>> {
        let conn = self.conn()?;
        let mut lib_stmt = conn.prepare("SELECT id FROM libraries ORDER BY id DESC LIMIT 1")?;
        let lib_id: Option<i64> = lib_stmt.query_row([], |r| r.get(0)).ok();
        let lib_id = match lib_id {
            Some(id) => id,
            None => return Ok(None),
        };

        let effective_folder_id = match folder_id {
            Some(id) if id > 0 => id,
            _ => {
                let mut root_stmt = conn.prepare("SELECT id FROM folders WHERE library_id = ?1 AND relative_path = ''")?;
                root_stmt.query_row(params![lib_id], |r| r.get(0)).unwrap_or(0)
            }
        };

        let current_folder = if effective_folder_id > 0 {
            self.get_folder_by_id_with_conn(&conn, effective_folder_id)?
        } else {
            None
        };

        let parent_folder = if let Some(ref cf) = current_folder {
            if let Some(pid) = cf.parent_id {
                self.get_folder_by_id_with_conn(&conn, pid)?
            } else {
                None
            }
        } else {
            None
        };

        let subfolders = if effective_folder_id > 0 {
            self.get_subfolders_with_conn(&conn, lib_id, Some(effective_folder_id))?
        } else {
            self.get_subfolders_with_conn(&conn, lib_id, None)?
        };

        let files = if effective_folder_id > 0 {
            self.get_folder_files_with_conn(&conn, effective_folder_id)?
        } else {
            Vec::new()
        };

        let breadcrumbs = self.get_breadcrumbs_with_conn(&conn, lib_id, current_folder.as_ref())?;

        Ok(Some(FolderContent {
            current_folder,
            parent_folder,
            subfolders,
            files,
            breadcrumbs,
        }))
    }

    pub fn update_folder_cover_status(&self, folder_id: i64, status: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE folders SET cover_status = ?1 WHERE id = ?2",
            params![status, folder_id],
        )?;
        Ok(())
    }

    pub fn batch_mark_folders_cover_ready(&self, folder_ids: &[i64]) -> Result<()> {
        if folder_ids.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare("UPDATE folders SET cover_status = 'ready' WHERE id = ?1")?;
            for &fid in folder_ids {
                stmt.execute(params![fid])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn reset_empty_folder_cover_statuses(&self, lib_id: i64) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE folders SET cover_status = 'unknown' WHERE library_id = ?1 AND cover_status = 'empty'",
            params![lib_id],
        )?;
        Ok(())
    }

    pub fn batch_upsert_folders(
        &self,
        lib_id: i64,
        root_folder_id: i64,
        folders: &[(PathBuf, String, String)],
    ) -> Result<(Vec<(i64, PathBuf)>, HashMap<String, i64>)> {
        let mut conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        let tx = conn.transaction()?;

        let mut path_to_id: HashMap<String, i64> = HashMap::with_capacity(folders.len() + 1);
        path_to_id.insert(String::new(), root_folder_id);

        // Preload existing folder ids for this library to avoid per-row SELECT
        {
            let mut existing_stmt = tx.prepare("SELECT relative_path, id FROM folders WHERE library_id = ?1")?;
            let rows = existing_stmt.query_map(params![lib_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            for row in rows.flatten() {
                path_to_id.insert(row.0, row.1);
            }
        }

        let mut upserted: Vec<(i64, PathBuf)> = Vec::with_capacity(folders.len());
        {
            let mut upsert_stmt = tx.prepare(
                r#"
                INSERT INTO folders (library_id, relative_path, name, parent_id, last_scanned_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(library_id, relative_path) DO UPDATE SET
                    name = excluded.name,
                    parent_id = excluded.parent_id,
                    last_scanned_at = excluded.last_scanned_at
                "#
            )?;
            let mut select_stmt = tx.prepare(
                "SELECT id FROM folders WHERE library_id = ?1 AND relative_path = ?2"
            )?;

            for (folder_path, rel_path, name) in folders {
                let parent_rel = match rel_path.rfind('/') {
                    Some(idx) => &rel_path[..idx],
                    None => "",
                };
                let parent_id = if parent_rel.is_empty() {
                    Some(root_folder_id)
                } else {
                    path_to_id.get(parent_rel).copied()
                };

                upsert_stmt.execute(params![lib_id, rel_path, name, parent_id, now])?;
                let fid = match path_to_id.get(rel_path).copied() {
                    Some(id) => id,
                    None => {
                        let id: i64 = select_stmt.query_row(params![lib_id, rel_path], |r| r.get(0))?;
                        path_to_id.insert(rel_path.clone(), id);
                        id
                    }
                };
                upserted.push((fid, folder_path.clone()));
            }
        }

        tx.commit()?;
        Ok((upserted, path_to_id))
    }

    pub fn batch_upsert_comic_files(
        &self,
        root_folder_id: i64,
        folder_map: &HashMap<String, i64>,
        files: &[(PathBuf, String, String, i64, String)],
    ) -> Result<Vec<(i64, PathBuf, String, bool)>> {
        let mut conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        let tx = conn.transaction()?;

        // Preload existing comic metadata to skip unchanged selects
        let mut existing_map: HashMap<String, (i64, String, i64)> = HashMap::with_capacity(files.len());
        {
            let mut existing_stmt = tx.prepare("SELECT relative_path, id, modified_at, size FROM comic_files")?;
            let rows = existing_stmt.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, (r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?)))
            })?;
            for row in rows.flatten() {
                existing_map.insert(row.0, row.1);
            }
        }

        let mut results: Vec<(i64, PathBuf, String, bool)> = Vec::with_capacity(files.len());
        {
            let mut update_stmt = tx.prepare(
                "UPDATE comic_files SET folder_id = ?1, name = ?2, extension = ?3, size = ?4, modified_at = ?5, status = 'active' WHERE id = ?6"
            )?;
            let mut insert_stmt = tx.prepare(
                r#"
                INSERT INTO comic_files (folder_id, relative_path, name, extension, size, modified_at, rating, comment, is_new, status, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, '', 1, 'active', ?7)
                "#
            )?;

            for (full_path, rel_path, name, size, mod_time) in files {
                let parent_rel = match rel_path.rfind('/') {
                    Some(idx) => &rel_path[..idx],
                    None => "",
                };
                let folder_id = if parent_rel.is_empty() {
                    root_folder_id
                } else {
                    folder_map.get(parent_rel).copied().unwrap_or(root_folder_id)
                };
                let ext = full_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                if let Some(&(id, ref old_mod, old_size)) = existing_map.get(rel_path) {
                    let file_changed = old_mod != mod_time || old_size != *size;
                    update_stmt.execute(params![folder_id, name, ext, size, mod_time, id])?;
                    results.push((id, full_path.clone(), mod_time.clone(), file_changed));
                } else {
                    insert_stmt.execute(params![folder_id, rel_path, name, ext, size, mod_time, now])?;
                    let id = tx.last_insert_rowid();
                    results.push((id, full_path.clone(), mod_time.clone(), true));
                }
            }
        }

        tx.commit()?;
        Ok(results)
    }

    pub fn update_folder_rating(&self, folder_id: i64, rating: i32) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("UPDATE folders SET rating = ?1 WHERE id = ?2", params![rating, folder_id])?;
        Ok(())
    }

    // --- Comic Files ---
    pub fn upsert_comic_file(
        &self,
        folder_id: i64,
        rel_path: &str,
        name: &str,
        ext: &str,
        size: i64,
        mod_time: &str,
    ) -> Result<(i64, bool)> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();

        let mut stmt = conn.prepare("SELECT id, modified_at, size, is_new FROM comic_files WHERE relative_path = ?1")?;
        let existing = stmt.query_row(params![rel_path], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, bool>(3)?))
        });

        match existing {
            Ok((id, old_mod, old_size, _is_new)) => {
                let file_changed = old_mod != mod_time || old_size != size;
                conn.execute(
                    "UPDATE comic_files SET folder_id = ?1, name = ?2, extension = ?3, size = ?4, modified_at = ?5, status = 'active' WHERE id = ?6",
                    params![folder_id, name, ext, size, mod_time, id],
                )?;
                Ok((id, file_changed))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                conn.execute(
                    r#"
                    INSERT INTO comic_files (folder_id, relative_path, name, extension, size, modified_at, rating, comment, is_new, status, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, '', 1, 'active', ?7)
                    "#,
                    params![folder_id, rel_path, name, ext, size, mod_time, now],
                )?;
                let id = conn.last_insert_rowid();
                Ok((id, true)) // newly inserted, needs cover extraction
            }
            Err(e) => Err(e),
        }
    }

    pub fn get_folder_files(&self, folder_id: i64) -> Result<Vec<ComicFileItem>> {
        let conn = self.conn()?;
        self.get_folder_files_with_conn(&conn, folder_id)
    }

    fn get_folder_files_with_conn(&self, conn: &Connection, folder_id: i64) -> Result<Vec<ComicFileItem>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT c.id, c.folder_id, c.relative_path, c.name, c.extension, c.size, c.modified_at,
                   c.rating, c.comment, c.is_new, c.status, cv.cache_path
            FROM comic_files c
            LEFT JOIN cover_caches cv ON c.id = cv.comic_file_id AND cv.status = 'ready'
            WHERE c.folder_id = ?1 AND c.status = 'active'
            ORDER BY c.name ASC
            "#
        )?;

        let rows = stmt.query_map(params![folder_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i32>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, bool>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, Option<String>>(11)?,
            ))
        })?;
        let raw_files: Vec<_> = rows.collect::<Result<Vec<_>, _>>()?;
        if raw_files.is_empty() {
            return Ok(Vec::new());
        }

        // Compute inherited tags ONCE for the folder instead of per file
        let inherited_tags = self.get_inherited_tags_for_folder(conn, folder_id)?;

        // Fetch all direct tags for all files in this folder in ONE batch query
        let mut direct_tag_map: HashMap<i64, Vec<TagItem>> = HashMap::new();
        let mut tag_stmt = conn.prepare(
            r#"
            SELECT ct.comic_file_id, t.id, t.name, t.display_name, t.is_system
            FROM comic_file_tags ct
            JOIN tags t ON ct.tag_id = t.id
            JOIN comic_files c ON ct.comic_file_id = c.id
            WHERE c.folder_id = ?1 AND c.status = 'active'
            ORDER BY t.display_name ASC
            "#
        )?;
        let tag_rows = tag_stmt.query_map(params![folder_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                TagItem {
                    id: r.get(1)?,
                    name: r.get(2)?,
                    display_name: r.get(3)?,
                    is_system: r.get(4)?,
                },
            ))
        })?;
        for tr in tag_rows {
            let (cid, tag) = tr?;
            direct_tag_map.entry(cid).or_default().push(tag);
        }

        let mut files = Vec::with_capacity(raw_files.len());
        for (id, fid, rel, name, ext, size, mod_at, rating, comment, is_new, status, cache_path) in raw_files {
            let direct_tags = direct_tag_map.remove(&id).unwrap_or_default();

            files.push(ComicFileItem {
                id,
                folder_id: fid,
                relative_path: rel,
                name,
                extension: ext,
                size,
                modified_at: mod_at,
                rating,
                comment,
                is_new,
                status,
                cover_cache_path: cache_path,
                cover_data_url: None,
                direct_tags,
                inherited_tags: inherited_tags.clone(),
            });
        }
        Ok(files)
    }

    pub fn get_comic_detail(&self, file_id: i64, lib_root: &str) -> Result<Option<ComicDetail>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT c.id, c.folder_id, f.name, f.rating, c.relative_path, c.name, c.extension,
                   c.size, c.modified_at, c.created_at, c.rating, c.comment, c.is_new, c.status
            FROM comic_files c
            JOIN folders f ON c.folder_id = f.id
            WHERE c.id = ?1
            "#
        )?;

        let mut rows = stmt.query(params![file_id])?;
        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let fid: i64 = row.get(1)?;
            let rel_path: String = row.get(4)?;
            let direct_tags = self.get_direct_comic_tags(&conn, id)?;
            let inherited_tags = self.get_inherited_tags_for_folder(&conn, fid)?;

            let full_path = Path::new(lib_root).join(&rel_path).to_string_lossy().to_string();

            Ok(Some(ComicDetail {
                id,
                folder_id: fid,
                folder_name: row.get(2)?,
                folder_rating: row.get(3)?,
                full_path,
                relative_path: rel_path,
                name: row.get(5)?,
                extension: row.get(6)?,
                size: row.get(7)?,
                modified_at: row.get(8)?,
                created_at: row.get(9)?,
                rating: row.get(10)?,
                comment: row.get(11)?,
                is_new: row.get(12)?,
                status: row.get(13)?,
                cover_data_url: None,
                direct_tags,
                inherited_tags,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_comic_rating(&self, file_id: i64, rating: i32) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("UPDATE comic_files SET rating = ?1 WHERE id = ?2", params![rating, file_id])?;
        Ok(())
    }

    pub fn update_comic_comment(&self, file_id: i64, comment: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("UPDATE comic_files SET comment = ?1 WHERE id = ?2", params![comment, file_id])?;
        Ok(())
    }

    pub fn mark_comic_as_read(&self, file_id: i64) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("UPDATE comic_files SET is_new = 0 WHERE id = ?1", params![file_id])?;
        Ok(())
    }

    pub fn toggle_comic_is_new(&self, file_id: i64) -> Result<bool> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE comic_files SET is_new = CASE WHEN is_new = 1 THEN 0 ELSE 1 END WHERE id = ?1",
            params![file_id],
        )?;
        let is_new: bool = conn.query_row(
            "SELECT is_new FROM comic_files WHERE id = ?1",
            params![file_id],
            |row| row.get(0),
        )?;
        Ok(is_new)
    }

    pub fn toggle_folder_is_new(&self, folder_id: i64) -> Result<bool> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE folders SET is_new = CASE WHEN is_new = 1 THEN 0 ELSE 1 END WHERE id = ?1",
            params![folder_id],
        )?;
        let is_new: bool = conn.query_row(
            "SELECT is_new FROM folders WHERE id = ?1",
            params![folder_id],
            |row| row.get(0),
        )?;
        let is_new_int = if is_new { 1 } else { 0 };
        let _ = conn.execute(
            "UPDATE comic_files SET is_new = ?1 WHERE folder_id = ?2",
            params![is_new_int, folder_id],
        );
        Ok(is_new)
    }

    pub fn delete_cover_cache(&self, file_id: i64) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM cover_caches WHERE comic_file_id = ?1", params![file_id])?;
        Ok(())
    }

    // --- Tags and Inheritance ---
    pub fn get_or_create_tag(&self, name: &str) -> Result<TagItem> {
        let conn = self.conn()?;
        let normalized = name.trim().trim_start_matches('#').to_lowercase();
        let display = name.trim().trim_start_matches('#').to_string();

        let mut stmt = conn.prepare("SELECT id, name, display_name, is_system FROM tags WHERE name = ?1")?;
        let existing = stmt.query_row(params![normalized], |row| {
            Ok(TagItem {
                id: row.get(0)?,
                name: row.get(1)?,
                display_name: row.get(2)?,
                is_system: row.get(3)?,
            })
        });

        match existing {
            Ok(t) => Ok(t),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                conn.execute(
                    "INSERT INTO tags (name, display_name, is_system) VALUES (?1, ?2, 0)",
                    params![normalized, display],
                )?;
                let id = conn.last_insert_rowid();
                Ok(TagItem {
                    id,
                    name: normalized,
                    display_name: display,
                    is_system: false,
                })
            }
            Err(e) => Err(e),
        }
    }

    pub fn add_tag_to_comic(&self, file_id: i64, tag_name: &str) -> Result<()> {
        let tag = self.get_or_create_tag(tag_name)?;
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO comic_file_tags (comic_file_id, tag_id) VALUES (?1, ?2)",
            params![file_id, tag.id],
        )?;
        Ok(())
    }

    pub fn remove_tag_from_comic(&self, file_id: i64, tag_id: i64) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM comic_file_tags WHERE comic_file_id = ?1 AND tag_id = ?2",
            params![file_id, tag_id],
        )?;
        Ok(())
    }

    pub fn add_tag_to_folder(&self, folder_id: i64, tag_name: &str) -> Result<()> {
        let tag = self.get_or_create_tag(tag_name)?;
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO folder_tags (folder_id, tag_id) VALUES (?1, ?2)",
            params![folder_id, tag.id],
        )?;
        Ok(())
    }

    pub fn remove_tag_from_folder(&self, folder_id: i64, tag_id: i64) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM folder_tags WHERE folder_id = ?1 AND tag_id = ?2",
            params![folder_id, tag_id],
        )?;
        Ok(())
    }

    pub fn get_direct_folder_tags(&self, conn: &Connection, folder_id: i64) -> Result<Vec<TagItem>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT t.id, t.name, t.display_name, t.is_system
            FROM tags t
            JOIN folder_tags ft ON t.id = ft.tag_id
            WHERE ft.folder_id = ?1
            ORDER BY t.display_name ASC
            "#
        )?;
        let rows = stmt.query_map(params![folder_id], |r| {
            Ok(TagItem {
                id: r.get(0)?,
                name: r.get(1)?,
                display_name: r.get(2)?,
                is_system: r.get(3)?,
            })
        })?;
        let mut tags = Vec::new();
        for t in rows {
            tags.push(t?);
        }
        Ok(tags)
    }

    pub fn get_direct_comic_tags(&self, conn: &Connection, file_id: i64) -> Result<Vec<TagItem>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT t.id, t.name, t.display_name, t.is_system
            FROM tags t
            JOIN comic_file_tags ct ON t.id = ct.tag_id
            WHERE ct.comic_file_id = ?1
            ORDER BY t.display_name ASC
            "#
        )?;
        let rows = stmt.query_map(params![file_id], |r| {
            Ok(TagItem {
                id: r.get(0)?,
                name: r.get(1)?,
                display_name: r.get(2)?,
                is_system: r.get(3)?,
            })
        })?;
        let mut tags = Vec::new();
        for t in rows {
            tags.push(t?);
        }
        Ok(tags)
    }

    // Recursive CTE for calculating inherited tags from all ancestor folders
    pub fn get_inherited_tags_for_folder(&self, conn: &Connection, folder_id: i64) -> Result<Vec<TagItem>> {
        let mut stmt = conn.prepare(
            r#"
            WITH RECURSIVE Ancestors(id, parent_id) AS (
                SELECT id, parent_id FROM folders WHERE id = ?1
                UNION ALL
                SELECT f.id, f.parent_id
                FROM folders f
                JOIN Ancestors a ON f.id = a.parent_id
                WHERE a.parent_id IS NOT NULL AND a.parent_id != 0
            )
            SELECT DISTINCT t.id, t.name, t.display_name, t.is_system
            FROM tags t
            JOIN folder_tags ft ON t.id = ft.tag_id
            JOIN Ancestors a ON ft.folder_id = a.id
            ORDER BY t.display_name ASC
            "#
        )?;

        let rows = stmt.query_map(params![folder_id], |r| {
            Ok(TagItem {
                id: r.get(0)?,
                name: r.get(1)?,
                display_name: r.get(2)?,
                is_system: r.get(3)?,
            })
        })?;
        let mut tags = Vec::new();
        for t in rows {
            tags.push(t?);
        }
        Ok(tags)
    }

    // --- Cover Cache ---
    pub fn upsert_cover_cache(&self, file_id: i64, cache_path: &str, file_mod_time: &str, status: &str, error: Option<&str>) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO cover_caches (comic_file_id, cache_path, file_modified_at, status, error)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(comic_file_id) DO UPDATE SET
                cache_path = excluded.cache_path,
                file_modified_at = excluded.file_modified_at,
                status = excluded.status,
                error = excluded.error
            "#,
            params![file_id, cache_path, file_mod_time, status, error],
        )?;
        Ok(())
    }

    pub fn get_cover_cache_path(&self, file_id: i64) -> Result<Option<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT cache_path FROM cover_caches WHERE comic_file_id = ?1 AND status = 'ready'")?;
        let mut rows = stmt.query(params![file_id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(r.get(0)?))
        } else {
            Ok(None)
        }
    }

    // --- Search Engine ---
    pub fn search_comics(&self, lib_id: i64, query: &SearchQuery) -> Result<Vec<ComicFileItem>> {
        let conn = self.conn()?;

        // Build dynamic WHERE clause based on keyword and tag logic
        let mut where_parts = vec!["c.status = 'active'".to_string()];
        where_parts.push("f.library_id = ?1".to_string());

        let mut param_index = 2;
        let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(lib_id)];

        // Rating filter
        if let Some(min_r) = query.min_rating {
            where_parts.push(format!("c.rating >= ?{}", param_index));
            query_params.push(Box::new(min_r));
            param_index += 1;
        }
        if let Some(max_r) = query.max_rating {
            where_parts.push(format!("c.rating <= ?{}", param_index));
            query_params.push(Box::new(max_r));
            param_index += 1;
        }

        // Extension filter
        if let Some(ref ext) = query.extension {
            if !ext.is_empty() && ext != "all" {
                where_parts.push(format!("c.extension = ?{}", param_index));
                query_params.push(Box::new(ext.to_lowercase()));
                param_index += 1;
            }
        }

        // Keyword group
        let mut keyword_cond = String::new();
        if !query.keywords.is_empty() {
            let mut kw_sub = Vec::new();
            for kw in &query.keywords {
                let pattern = format!("%{}%", kw);
                kw_sub.push(format!("(c.name LIKE ?{} OR f.name LIKE ?{})", param_index, param_index));
                query_params.push(Box::new(pattern));
                param_index += 1;
            }
            let joiner = if query.keyword_operator.to_uppercase() == "OR" { " OR " } else { " AND " };
            keyword_cond = format!("({})", kw_sub.join(joiner));
        }

        // Tag group (searches direct tags and inherited ancestor folder tags)
        let mut tag_cond = String::new();
        if !query.tag_names.is_empty() {
            let mut tag_sub = Vec::new();
            for t_name in &query.tag_names {
                let norm = t_name.trim().trim_start_matches('#').to_lowercase();
                let subquery = format!(
                    r#"
                    (
                        EXISTS (
                            SELECT 1 FROM comic_file_tags cft
                            JOIN tags t ON cft.tag_id = t.id
                            WHERE cft.comic_file_id = c.id AND t.name = ?{0}
                        )
                        OR
                        EXISTS (
                            WITH RECURSIVE Anc(id, parent_id) AS (
                                SELECT id, parent_id FROM folders WHERE id = c.folder_id
                                UNION ALL
                                SELECT f2.id, f2.parent_id FROM folders f2 JOIN Anc ON f2.id = Anc.parent_id
                            )
                            SELECT 1 FROM folder_tags ft
                            JOIN tags t ON ft.tag_id = t.id
                            JOIN Anc a ON ft.folder_id = a.id
                            WHERE t.name = ?{0}
                        )
                    )
                    "#,
                    param_index
                );
                tag_sub.push(subquery);
                query_params.push(Box::new(norm));
                param_index += 1;
            }
            let joiner = if query.tag_operator.to_uppercase() == "OR" { " OR " } else { " AND " };
            tag_cond = format!("({})", tag_sub.join(joiner));
        }

        // Combine keyword and tag conditions with group_operator
        if !keyword_cond.is_empty() && !tag_cond.is_empty() {
            let group_joiner = if query.group_operator.to_uppercase() == "OR" { " OR " } else { " AND " };
            where_parts.push(format!("({} {} {})", keyword_cond, group_joiner, tag_cond));
        } else if !keyword_cond.is_empty() {
            where_parts.push(keyword_cond);
        } else if !tag_cond.is_empty() {
            where_parts.push(tag_cond);
        }

        // Order by
        let order_col = match query.sort_by.as_str() {
            "rating" => "c.rating",
            "date" => "c.created_at",
            "modified" => "c.modified_at",
            "size" => "c.size",
            _ => "c.name",
        };
        let order_dir = if query.sort_order.to_lowercase() == "desc" { "DESC" } else { "ASC" };

        let sql = format!(
            r#"
            SELECT c.id, c.folder_id, c.relative_path, c.name, c.extension, c.size, c.modified_at,
                   c.rating, c.comment, c.is_new, c.status, cv.cache_path
            FROM comic_files c
            JOIN folders f ON c.folder_id = f.id
            LEFT JOIN cover_caches cv ON c.id = cv.comic_file_id AND cv.status = 'ready'
            WHERE {}
            ORDER BY {} {}
            LIMIT 500
            "#,
            where_parts.join(" AND "),
            order_col,
            order_dir
        );

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = query_params.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(&param_refs[..], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i32>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, bool>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, Option<String>>(11)?,
            ))
        })?;

        let mut files = Vec::new();
        for r in rows {
            let (id, fid, rel, name, ext, size, mod_at, rating, comment, is_new, status, cache_path) = r?;
            let direct_tags = self.get_direct_comic_tags(&conn, id)?;
            let inherited_tags = self.get_inherited_tags_for_folder(&conn, fid)?;

            files.push(ComicFileItem {
                id,
                folder_id: fid,
                relative_path: rel,
                name,
                extension: ext,
                size,
                modified_at: mod_at,
                rating,
                comment,
                is_new,
                status,
                cover_cache_path: cache_path,
                cover_data_url: None,
                direct_tags,
                inherited_tags,
            });
        }
        Ok(files)
    }

    pub fn search_folders(&self, lib_id: i64, query: &SearchQuery) -> Result<Vec<FolderItem>> {
        let conn = self.conn()?;

        let mut where_parts = vec!["f.library_id = ?1".to_string(), "f.relative_path != ''".to_string()];
        let mut param_index = 2;
        let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(lib_id)];

        // Rating filter
        if let Some(min_r) = query.min_rating {
            where_parts.push(format!("f.rating >= ?{}", param_index));
            query_params.push(Box::new(min_r));
            param_index += 1;
        }
        if let Some(max_r) = query.max_rating {
            where_parts.push(format!("f.rating <= ?{}", param_index));
            query_params.push(Box::new(max_r));
            param_index += 1;
        }

        // Keywords filter on folder name
        let mut keyword_cond = String::new();
        if !query.keywords.is_empty() {
            let mut kw_sub = Vec::new();
            for kw in &query.keywords {
                let pattern = format!("%{}%", kw);
                kw_sub.push(format!("f.name LIKE ?{}", param_index));
                query_params.push(Box::new(pattern));
                param_index += 1;
            }
            let joiner = if query.keyword_operator.to_uppercase() == "OR" { " OR " } else { " AND " };
            keyword_cond = format!("({})", kw_sub.join(joiner));
        }

        // Tag filter: check if folder has this tag directly OR inherited from ancestor folder
        let mut tag_cond = String::new();
        if !query.tag_names.is_empty() {
            let mut tag_sub = Vec::new();
            for t_name in &query.tag_names {
                let norm = t_name.trim().trim_start_matches('#').to_lowercase();
                let subquery = format!(
                    r#"
                    EXISTS (
                        WITH RECURSIVE Anc(id, parent_id) AS (
                            SELECT id, parent_id FROM folders WHERE id = f.id
                            UNION ALL
                            SELECT f2.id, f2.parent_id FROM folders f2 JOIN Anc ON f2.id = Anc.parent_id
                        )
                        SELECT 1 FROM folder_tags ft
                        JOIN tags t ON ft.tag_id = t.id
                        JOIN Anc a ON ft.folder_id = a.id
                        WHERE t.name = ?{0}
                    )
                    "#,
                    param_index
                );
                tag_sub.push(subquery);
                query_params.push(Box::new(norm));
                param_index += 1;
            }
            let joiner = if query.tag_operator.to_uppercase() == "OR" { " OR " } else { " AND " };
            tag_cond = format!("({})", tag_sub.join(joiner));
        }

        // Combine keyword and tag conditions
        if !keyword_cond.is_empty() && !tag_cond.is_empty() {
            let group_joiner = if query.group_operator.to_uppercase() == "OR" { " OR " } else { " AND " };
            where_parts.push(format!("({} {} {})", keyword_cond, group_joiner, tag_cond));
        } else if !keyword_cond.is_empty() {
            where_parts.push(keyword_cond);
        } else if !tag_cond.is_empty() {
            where_parts.push(tag_cond);
        }

        // Order
        let order_col = match query.sort_by.as_str() {
            "rating" => "f.rating",
            "date" => "f.created_at",
            _ => "f.name",
        };
        let order_dir = if query.sort_order.to_lowercase() == "desc" { "DESC" } else { "ASC" };

        let sql = format!(
            r#"
            SELECT f.id, f.library_id, f.relative_path, f.name, f.parent_id, f.rating, f.is_new, f.cover_status
            FROM folders f
            WHERE {}
            ORDER BY {} {}
            LIMIT 200
            "#,
            where_parts.join(" AND "),
            order_col,
            order_dir
        );

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = query_params.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(&param_refs[..], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, bool>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;

        let mut raw_folders = Vec::new();
        for r in rows {
            raw_folders.push(r?);
        }

        if raw_folders.is_empty() {
            return Ok(Vec::new());
        }

        // Batch load direct tags for the matching folders
        let f_ids: Vec<i64> = raw_folders.iter().map(|f| f.0).collect();
        let placeholders: Vec<String> = f_ids.iter().map(|_| "?".to_string()).collect();
        let tag_sql = format!(
            r#"
            SELECT ft.folder_id, t.id, t.name, t.display_name, t.is_system
            FROM folder_tags ft
            JOIN tags t ON ft.tag_id = t.id
            WHERE ft.folder_id IN ({})
            ORDER BY t.display_name ASC
            "#,
            placeholders.join(",")
        );
        let mut tag_stmt = conn.prepare(&tag_sql)?;
        let f_id_params: Vec<&dyn rusqlite::ToSql> = f_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        let tag_rows = tag_stmt.query_map(&f_id_params[..], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                TagItem {
                    id: row.get(1)?,
                    name: row.get(2)?,
                    display_name: row.get(3)?,
                    is_system: row.get(4)?,
                },
            ))
        })?;

        let mut direct_tag_map: HashMap<i64, Vec<TagItem>> = HashMap::new();
        for tr in tag_rows {
            let (fid, tag) = tr?;
            direct_tag_map.entry(fid).or_default().push(tag);
        }

        let mut folders = Vec::with_capacity(raw_folders.len());
        for (id, lib_id, rel_path, name, parent_id, rating, is_new, cover_status) in raw_folders {
            let direct_tags = direct_tag_map.remove(&id).unwrap_or_default();
            folders.push(FolderItem {
                id,
                library_id: lib_id,
                relative_path: rel_path,
                name,
                parent_id,
                rating,
                is_new,
                cover_status,
                cover_data_url: None,
                direct_tags,
            });
        }

        Ok(folders)
    }

    // --- Tag Management ---
    pub fn update_tag(&self, id: i64, new_display_name: &str) -> Result<()> {
        let conn = self.conn()?;
        let normalized = new_display_name.trim().trim_start_matches('#').to_lowercase();
        let display = new_display_name.trim().trim_start_matches('#').to_string();
        conn.execute(
            "UPDATE tags SET name = ?1, display_name = ?2 WHERE id = ?3",
            params![normalized, display, id],
        )?;
        Ok(())
    }

    pub fn delete_tag(&self, id: i64) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM tags WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_tags_with_counts(&self) -> Result<Vec<crate::models::TagDetailItem>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT t.id, t.name, t.display_name, t.is_system,
                   (SELECT COUNT(*) FROM comic_file_tags WHERE tag_id = t.id) as f_cnt,
                   (SELECT COUNT(*) FROM folder_tags WHERE tag_id = t.id) as d_cnt
            FROM tags t
            ORDER BY t.display_name ASC
            "#
        )?;

        let rows = stmt.query_map([], |r| {
            Ok(crate::models::TagDetailItem {
                id: r.get(0)?,
                name: r.get(1)?,
                display_name: r.get(2)?,
                is_system: r.get(3)?,
                file_count: r.get(4)?,
                folder_count: r.get(5)?,
            })
        })?;

        let mut tags = Vec::new();
        for t in rows {
            tags.push(t?);
        }
        Ok(tags)
    }

    pub fn get_folder_detail(&self, folder_id: i64, lib_root: &str) -> Result<Option<crate::models::FolderDetail>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT f.id, f.library_id, f.relative_path, f.name, f.parent_id, f.rating, f.is_new,
                   (SELECT COUNT(*) FROM folders sub WHERE sub.parent_id = f.id) as subfolder_cnt,
                   (SELECT COUNT(*) FROM comic_files cf WHERE cf.folder_id = f.id AND cf.status = 'active') as file_cnt
            FROM folders f
            WHERE f.id = ?1
            "#
        )?;

        let mut rows = stmt.query(params![folder_id])?;
        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let rel_path: String = row.get(2)?;
            let tags = self.get_direct_folder_tags(&conn, id)?;

            let full_path = if rel_path.is_empty() {
                lib_root.to_string()
            } else {
                std::path::Path::new(lib_root).join(&rel_path).to_string_lossy().to_string()
            };

            Ok(Some(crate::models::FolderDetail {
                id,
                library_id: row.get(1)?,
                relative_path: rel_path,
                full_path,
                name: row.get(3)?,
                parent_id: row.get(4)?,
                rating: row.get(5)?,
                is_new: row.get(6)?,
                cover_data_url: None,
                direct_tags: tags,
                subfolder_count: row.get::<_, i64>(7)? as usize,
                file_count: row.get::<_, i64>(8)? as usize,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_root_folder_id(&self, lib_id: i64) -> Result<Option<i64>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT id FROM folders WHERE library_id = ?1 AND relative_path = ''")?;
        Ok(stmt.query_row(params![lib_id], |r| r.get(0)).ok())
    }

    pub fn get_breadcrumbs(&self, lib_id: i64, current_folder: Option<&FolderItem>) -> Result<Vec<BreadcrumbItem>> {
        let conn = self.conn()?;
        self.get_breadcrumbs_with_conn(&conn, lib_id, current_folder)
    }

    fn get_breadcrumbs_with_conn(&self, conn: &Connection, lib_id: i64, current_folder: Option<&FolderItem>) -> Result<Vec<BreadcrumbItem>> {
        let mut breadcrumbs = vec![BreadcrumbItem {
            id: None,
            name: "라이브러리 홈".to_string(),
            relative_path: "".to_string(),
        }];

        if let Some(cf) = current_folder {
            if !cf.relative_path.is_empty() {
                let mut stmt = conn.prepare("SELECT id FROM folders WHERE library_id = ?1 AND relative_path = ?2")?;
                let parts: Vec<&str> = cf.relative_path.split('/').collect();
                let mut accumulated = String::new();
                for part in parts {
                    if !accumulated.is_empty() {
                        accumulated.push('/');
                    }
                    accumulated.push_str(part);
                    let f_id = stmt.query_row(params![lib_id, &accumulated], |r| r.get(0)).ok();
                    breadcrumbs.push(BreadcrumbItem {
                        id: f_id,
                        name: part.to_string(),
                        relative_path: accumulated.clone(),
                    });
                }
            }
        }
        Ok(breadcrumbs)
    }

    /// Returns `(folder_rel_path, first_comic_file_id, child_folder_rel_paths)` for resolving a folder cover.
    pub fn get_folder_cover_candidates(&self, folder_id: i64) -> Result<(Option<String>, Option<i64>, Vec<String>)> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT library_id, relative_path FROM folders WHERE id = ?1")?;
        let folder_info: Option<(i64, String)> = stmt
            .query_row(params![folder_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .ok();

        let (lib_id, rel_path) = match folder_info {
            Some(info) => info,
            None => return Ok((None, None, Vec::new())),
        };

        if rel_path.is_empty() {
            return Ok((Some(rel_path), None, Vec::new()));
        }

        // 1. Check direct comic file in this folder first (uses idx_comic_files_folder)
        let mut direct_stmt = conn.prepare(
            "SELECT id FROM comic_files WHERE folder_id = ?1 AND status = 'active' ORDER BY name ASC LIMIT 1"
        )?;
        let mut first_comic_id: Option<i64> = direct_stmt.query_row(params![folder_id], |r| r.get(0)).ok();

        // 2. If none, check descendant folders using binary B-tree prefix range on UNIQUE(library_id, relative_path)
        if first_comic_id.is_none() {
            let prefix_start = format!("{}/", rel_path);
            let prefix_end = format!("{}0", rel_path);
            let mut desc_stmt = conn.prepare(
                r#"
                SELECT c.id
                FROM folders f
                JOIN comic_files c ON c.folder_id = f.id
                WHERE f.library_id = ?1
                  AND f.relative_path >= ?2
                  AND f.relative_path < ?3
                  AND c.status = 'active'
                ORDER BY f.relative_path ASC, c.name ASC
                LIMIT 1
                "#
            )?;
            first_comic_id = desc_stmt
                .query_row(params![lib_id, &prefix_start, &prefix_end], |r| r.get(0))
                .ok();
        }

        Ok((Some(rel_path), first_comic_id, Vec::new()))
    }

    pub fn rebuild_database(&self) -> Result<()> {
        let conn = self.conn()?;
        conn.execute_batch(
            r#"
            DELETE FROM cover_caches;
            DELETE FROM comic_file_tags;
            DELETE FROM folder_tags;
            DELETE FROM comic_files;
            DELETE FROM folders;
            DELETE FROM tags;
            DELETE FROM libraries;
            VACUUM;
            "#
        )?;
        Ok(())
    }

    // --- Statistics & All Tags ---
    pub fn get_all_tags(&self) -> Result<Vec<TagItem>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT id, name, display_name, is_system FROM tags ORDER BY display_name ASC")?;
        let rows = stmt.query_map([], |r| {
            Ok(TagItem {
                id: r.get(0)?,
                name: r.get(1)?,
                display_name: r.get(2)?,
                is_system: r.get(3)?,
            })
        })?;
        let mut tags = Vec::new();
        for t in rows {
            tags.push(t?);
        }
        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Database {
        let temp_dir = std::env::temp_dir().join(format!("test_comics_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_file = temp_dir.join("test.db");
        Database::new(db_file).unwrap()
    }

    #[test]
    fn test_tag_inheritance_and_rating_independence() {
        let db = setup_test_db();
        let lib = db.get_or_create_library("D:/TestLibrary").unwrap();

        // 1. Root folder
        let root_id = db.upsert_folder(lib.id, "", "Root", None).unwrap();
        // 2. Parent folder (e.g. Manga/OnePiece)
        let parent_id = db.upsert_folder(lib.id, "Manga", "Manga", Some(root_id)).unwrap();
        let series_id = db.upsert_folder(lib.id, "Manga/OnePiece", "OnePiece", Some(parent_id)).unwrap();

        // Add folder tag to Manga
        db.add_tag_to_folder(parent_id, "소년물").unwrap();
        // Add folder tag to OnePiece
        db.add_tag_to_folder(series_id, "해적").unwrap();
        // Set folder rating
        db.update_folder_rating(series_id, 4).unwrap();

        // Add comic file in OnePiece
        let (file_id, _) = db.upsert_comic_file(
            series_id,
            "Manga/OnePiece/vol1.cbz",
            "vol1.cbz",
            "cbz",
            1024000,
            "2026-10-24T00:00:00Z",
        ).unwrap();

        // Add direct tag to file
        db.add_tag_to_comic(file_id, "1화").unwrap();
        // Set file rating
        db.update_comic_rating(file_id, 5).unwrap();

        // Detail check
        let detail = db.get_comic_detail(file_id, &lib.root_path).unwrap().unwrap();

        // Rating independence check
        assert_eq!(detail.rating, 5, "파일 별점은 5점이어야 함");
        assert_eq!(detail.folder_rating, 4, "상위 폴더 별점은 4점이어야 함");

        // Direct tags check
        let direct_tag_names: Vec<String> = detail.direct_tags.into_iter().map(|t| t.name).collect();
        assert_eq!(direct_tag_names, vec!["1화"]);

        // Inherited tags check: Should have '소년물' (from Manga) and '해적' (from OnePiece)
        let inherited_tag_names: Vec<String> = detail.inherited_tags.into_iter().map(|t| t.name).collect();
        assert!(inherited_tag_names.contains(&"소년물".to_string()), "Manga 폴더의 '소년물' 태그가 상속되어야 함");
        assert!(inherited_tag_names.contains(&"해적".to_string()), "OnePiece 폴더의 '해적' 태그가 상속되어야 함");

        // Search test 1: Searching comics by inherited tag '소년물' finds the comic file
        let search_query = SearchQuery {
            keywords: vec![],
            keyword_operator: "AND".to_string(),
            tag_names: vec!["소년물".to_string()],
            tag_operator: "AND".to_string(),
            group_operator: "AND".to_string(),
            min_rating: None,
            max_rating: None,
            extension: None,
            sort_by: "name".to_string(),
            sort_order: "asc".to_string(),
            include_folder_tags: None,
        };
        let results = db.search_comics(lib.id, &search_query).unwrap();
        assert_eq!(results.len(), 1, "상속된 태그 '소년물'로 만화책 검색 시 1건이 검색되어야 함");
        assert_eq!(results[0].id, file_id);

        // Search test 2: Searching folders by tag '해적' finds OnePiece folder itself
        let mut folder_search_query = search_query.clone();
        folder_search_query.tag_names = vec!["해적".to_string()];
        let folder_results = db.search_folders(lib.id, &folder_search_query).unwrap();
        assert_eq!(folder_results.len(), 1, "'해적' 태그로 폴더 검색 시 OnePiece 폴더가 검색되어야 함");
        assert_eq!(folder_results[0].id, series_id);

        // Search test 3: Searching folders by file-only tag '1화' returns 0 folders
        let mut file_only_query = search_query.clone();
        file_only_query.tag_names = vec!["1화".to_string()];
        let zero_folders = db.search_folders(lib.id, &file_only_query).unwrap();
        assert_eq!(zero_folders.len(), 0, "파일 고유 태그로 폴더 검색 시 0건이어야 함");

        // Rating filter test: min_rating = 5 should find it, min_rating = 6 should not
        let mut rating_query = search_query.clone();
        rating_query.min_rating = Some(5);
        let results_r5 = db.search_comics(lib.id, &rating_query).unwrap();
        assert_eq!(results_r5.len(), 1);

        rating_query.min_rating = Some(6);
        let results_r6 = db.search_comics(lib.id, &rating_query).unwrap();
        assert_eq!(results_r6.len(), 0);
    }

    #[test]
    fn test_tag_management_and_rebuild() {
        let db = setup_test_db();
        let lib = db.get_or_create_library("D:/TestLibrary2").unwrap();
        let root_id = db.upsert_folder(lib.id, "", "Root", None).unwrap();

        let (file_id, _) = db.upsert_comic_file(
            root_id,
            "sample.cbz",
            "sample.cbz",
            "cbz",
            5000,
            "2026-10-24T00:00:00Z",
        ).unwrap();

        // 1. Tag creation and attach
        let tag = db.get_or_create_tag("판타지").unwrap();
        db.add_tag_to_comic(file_id, "판타지").unwrap();

        let counts = db.get_tags_with_counts().unwrap();
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].name, "판타지");
        assert_eq!(counts[0].file_count, 1);
        assert_eq!(counts[0].folder_count, 0);

        // 2. Tag rename
        db.update_tag(tag.id, "정통판타지").unwrap();
        let tags_after_rename = db.get_all_tags().unwrap();
        assert_eq!(tags_after_rename[0].display_name, "정통판타지");
        assert_eq!(tags_after_rename[0].name, "정통판타지");

        // 3. Tag delete
        db.delete_tag(tag.id).unwrap();
        let tags_after_delete = db.get_all_tags().unwrap();
        assert_eq!(tags_after_delete.len(), 0);

        // 4. Rebuild Database
        db.rebuild_database().unwrap();
        let empty_lib = db.get_latest_library().unwrap();
        assert!(empty_lib.is_none(), "DB 재구축 후 라이브러리 목록이 비어있어야 함");
    }

    #[test]
    fn test_legacy_db_migration_without_is_new() {
        let temp_dir = std::env::temp_dir().join(format!("test_legacy_db_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("legacy.db");

        // Manually create an old DB where folders table lacks is_new column
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE libraries (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    root_path TEXT NOT NULL UNIQUE,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    last_scanned_at TEXT
                );
                CREATE TABLE folders (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    library_id INTEGER NOT NULL,
                    relative_path TEXT NOT NULL,
                    name TEXT NOT NULL,
                    parent_id INTEGER,
                    rating INTEGER NOT NULL DEFAULT 0,
                    last_scanned_at TEXT,
                    UNIQUE(library_id, relative_path)
                );
                INSERT INTO libraries (root_path, created_at, updated_at) VALUES ('D:/OldLib', 'now', 'now');
                INSERT INTO folders (library_id, relative_path, name, parent_id, rating) VALUES (1, '', 'Root', NULL, 0);
                INSERT INTO folders (library_id, relative_path, name, parent_id, rating) VALUES (1, 'sub', 'Sub', 1, 3);
                "#
            ).unwrap();
        }

        // Now initialize Database with this legacy db
        let db = Database::new(db_path.clone());
        assert!(db.is_ok(), "Database::new must successfully migrate legacy DB: {:?}", db.err());

        let db = db.unwrap();
        let folder = db.get_folder_by_id(2).unwrap().unwrap();
        assert_eq!(folder.name, "Sub");
        assert_eq!(folder.rating, 3);
        assert!(folder.is_new, "Migrated folder must have is_new = true by default");
        assert_eq!(folder.cover_status, "unknown");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_folder_content_bundle_and_cover_candidates() {
        let db = setup_test_db();
        let lib = db.get_or_create_library("D:/TestBundle").unwrap();
        let root_id = db.upsert_folder(lib.id, "", "Root", None).unwrap();

        let folders_input = vec![
            (PathBuf::from("D:/TestBundle/Manga"), "Manga".to_string(), "Manga".to_string()),
            (PathBuf::from("D:/TestBundle/Manga2"), "Manga2".to_string(), "Manga2".to_string()),
            (PathBuf::from("D:/TestBundle/Manga/Sub"), "Manga/Sub".to_string(), "Sub".to_string()),
        ];
        let (upserted, folder_map) = db.batch_upsert_folders(lib.id, root_id, &folders_input).unwrap();
        assert_eq!(upserted.len(), 3);
        let manga_id = folder_map["Manga"];
        let manga2_id = folder_map["Manga2"];
        let sub_id = folder_map["Manga/Sub"];

        let files_input = vec![
            (PathBuf::from("D:/TestBundle/Manga2/other.cbz"), "Manga2/other.cbz".to_string(), "other.cbz".to_string(), 100, "t1".to_string()),
            (PathBuf::from("D:/TestBundle/Manga/Sub/ch1.cbz"), "Manga/Sub/ch1.cbz".to_string(), "ch1.cbz".to_string(), 200, "t2".to_string()),
        ];
        let upserted_files = db.batch_upsert_comic_files(root_id, &folder_map, &files_input).unwrap();
        assert_eq!(upserted_files.len(), 2);
        let ch1_id = upserted_files[1].0;

        // Manga has no direct comic, only descendant in Manga/Sub; must NOT match Manga2/other.cbz
        let (_, cand_id, _) = db.get_folder_cover_candidates(manga_id).unwrap();
        assert_eq!(cand_id, Some(ch1_id));

        // Cover status transitions
        db.update_folder_cover_status(manga2_id, "empty").unwrap();
        db.batch_mark_folders_cover_ready(&[sub_id]).unwrap();
        let bundle = db.get_folder_content_bundle(None).unwrap().unwrap();
        assert_eq!(bundle.subfolders.len(), 2);
        assert_eq!(bundle.subfolders[0].id, manga_id);
        assert_eq!(bundle.subfolders[0].cover_status, "unknown");
        assert_eq!(bundle.subfolders[1].id, manga2_id);
        assert_eq!(bundle.subfolders[1].cover_status, "empty");

        db.reset_empty_folder_cover_statuses(lib.id).unwrap();
        let reloaded_m2 = db.get_folder_by_id(manga2_id).unwrap().unwrap();
        assert_eq!(reloaded_m2.cover_status, "unknown");
    }
}
