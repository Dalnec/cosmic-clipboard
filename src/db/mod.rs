pub mod models;

use models::{ClipboardEntry, ItemType};
use rusqlite::{Connection, Result, params, OptionalExtension};
use std::path::Path;
use std::fs;

const DATABASE_VERSION: i32 = 2;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        if let Some(parent) = db_path.as_ref().parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).expect("Failed to create database directories");
            }
        }
        
        let conn = Connection::open(db_path)?;
        let mut db = Self { conn };
        db.init()?;
        Ok(db)
    }

    pub fn init(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;

        tx.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_version (
                id integer PRIMARY KEY CHECK (id = 1),
                version integer
            )",
            [],
        )?;

        let mut current_version: i32 = tx.query_row(
            "SELECT version FROM clipboard_version WHERE id = 1",
            [],
            |row| row.get(0),
        ).optional()?.unwrap_or(0);

        if current_version == 0 {
            tx.execute("INSERT OR IGNORE INTO clipboard_version (id, version) VALUES (1, 0)", [])?;
        }

        // Migrations
        if current_version == 0 {
            tx.execute(
                "CREATE TABLE IF NOT EXISTS clipboard (
                    id       integer   NOT NULL UNIQUE PRIMARY KEY AUTOINCREMENT,
                    type     text      NOT NULL,
                    content  text      NOT NULL,
                    pinned   boolean   NOT NULL,
                    tag      text,
                    datetime timestamp NOT NULL,
                    metadata text,
                    UNIQUE (type, content)
                )",
                [],
            )?;
            current_version = 1;
        }

        if current_version == 1 {
            // Esta columna se agregó en la v2 de TS
            let _ = tx.execute("ALTER TABLE clipboard ADD COLUMN title text", []);
            current_version = 2;
        }

        if current_version != DATABASE_VERSION {
            tx.execute("UPDATE clipboard_version SET version = ? WHERE id = 1", params![DATABASE_VERSION])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn insert_entry(&self, entry: &ClipboardEntry) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO clipboard (type, content, pinned, tag, datetime, metadata, title)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(type, content) DO UPDATE SET datetime=excluded.datetime",
            params![
                entry.item_type.to_string(),
                entry.content,
                entry.pinned,
                entry.tag,
                entry.datetime,
                entry.metadata,
                entry.title,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_entries(&self) -> Result<Vec<ClipboardEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type, content, pinned, tag, datetime, metadata, title
             FROM clipboard ORDER BY pinned DESC, datetime DESC"
        )?;

        let entry_iter = stmt.query_map([], |row| {
            let item_type_str: String = row.get(1)?;
            let item_type: ItemType = item_type_str.parse().unwrap_or(ItemType::Text);

            Ok(ClipboardEntry {
                id: Some(row.get(0)?),
                item_type,
                content: row.get(2)?,
                pinned: row.get(3)?,
                tag: row.get(4)?,
                datetime: row.get(5)?,
                metadata: row.get(6)?,
                title: row.get(7)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in entry_iter {
            entries.push(entry?);
        }

        Ok(entries)
    }

    #[allow(dead_code)]
    pub fn delete_entry(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM clipboard WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_entry_metadata(&self, id: i64, pinned: bool, tag: Option<String>, title: Option<String>) -> Result<()> {
        self.conn.execute(
            "UPDATE clipboard SET pinned = ?1, tag = ?2, title = ?3 WHERE id = ?4",
            params![pinned, tag, title, id],
        )?;
        Ok(())
    }
}
