//! Local SQLite storage: clipboard history (encrypted at rest), snippets, counters,
//! and content-free action usage stats.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use minpoint_core::snippets::Snippet;
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Mutex;

pub type KeyProvider = Box<dyn Fn() -> Result<[u8; 32], String> + Send + Sync>;

pub struct Db {
    conn: Mutex<Connection>,
    /// The encryption key lives in the OS credential store and is fetched only the first
    /// time clipboard content is read or written — never during app startup, where a
    /// Keychain prompt or a slow Credential Manager would freeze the UI.
    key_provider: KeyProvider,
    cipher: std::sync::OnceLock<Result<Aes256Gcm, String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: i64,
    pub kind: String,
    pub text: String,
    pub app: String,
    pub created_at: i64,
    pub pinned: bool,
    pub favorite: bool,
}

const MIGRATIONS: &[&str] = &[
    // v1
    "CREATE TABLE clipboard_items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        kind TEXT NOT NULL,
        content BLOB NOT NULL,
        content_hash TEXT NOT NULL,
        app TEXT NOT NULL DEFAULT '',
        created_at INTEGER NOT NULL,
        pinned INTEGER NOT NULL DEFAULT 0,
        favorite INTEGER NOT NULL DEFAULT 0
     );
     CREATE INDEX clipboard_created ON clipboard_items(created_at);
     CREATE UNIQUE INDEX clipboard_hash ON clipboard_items(content_hash);
     CREATE TABLE snippets (id TEXT PRIMARY KEY, data TEXT NOT NULL, updated_at INTEGER NOT NULL);
     CREATE TABLE counters (name TEXT PRIMARY KEY, value INTEGER NOT NULL);
     CREATE TABLE action_usage (action_id TEXT PRIMARY KEY, count INTEGER NOT NULL, last_used INTEGER NOT NULL);",
];

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

impl Db {
    /// `key` is a 32-byte key held in the OS credential store.
    pub fn open(path: &Path, key_provider: KeyProvider) -> Result<Db, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA secure_delete=ON;").map_err(|e| e.to_string())?;
        let ver: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(|e| e.to_string())?;
        for (i, m) in MIGRATIONS.iter().enumerate().skip(ver as usize) {
            conn.execute_batch(&format!("BEGIN; {m}; PRAGMA user_version = {}; COMMIT;", i + 1)).map_err(|e| format!("migration {}: {e}", i + 1))?;
        }
        Ok(Db { conn: Mutex::new(conn), key_provider, cipher: std::sync::OnceLock::new() })
    }

    /// Fallback when the database file can't be opened: data lasts for this session only.
    pub fn open_temporary(key_provider: KeyProvider) -> Db {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        for m in MIGRATIONS {
            conn.execute_batch(m).expect("migrations");
        }
        Db { conn: Mutex::new(conn), key_provider, cipher: std::sync::OnceLock::new() }
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Db {
        let conn = Connection::open_in_memory().unwrap();
        for m in MIGRATIONS {
            conn.execute_batch(m).unwrap();
        }
        Db { conn: Mutex::new(conn), key_provider: Box::new(|| Ok([7u8; 32])), cipher: std::sync::OnceLock::new() }
    }

    fn cipher(&self) -> Result<&Aes256Gcm, String> {
        self.cipher
            .get_or_init(|| {
                let key = (self.key_provider)()?;
                Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())
            })
            .as_ref()
            .map_err(|e| format!("Clipboard history is unavailable: {e}"))
    }

    fn seal(&self, s: &str) -> Result<Vec<u8>, String> {
        let mut nonce = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        let mut out = nonce.to_vec();
        out.extend(self.cipher()?.encrypt(Nonce::from_slice(&nonce), s.as_bytes()).map_err(|_| "encryption failed".to_string())?);
        Ok(out)
    }

    fn open_blob(&self, b: &[u8]) -> Option<String> {
        if b.len() < 12 {
            return None;
        }
        let pt = self.cipher().ok()?.decrypt(Nonce::from_slice(&b[..12]), &b[12..]).ok()?;
        String::from_utf8(pt).ok()
    }

    fn hash(&self, kind: &str, s: &str) -> String {
        // Keyed hash so the DB doesn't reveal content via unsalted digests.
        let mut h = Sha256::new();
        h.update(b"minpoint-clip-v1");
        h.update(kind.as_bytes());
        h.update([0]);
        h.update(s.as_bytes());
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    // ---- clipboard ----

    /// Insert or bump an item; duplicates move to the top instead of being stored twice.
    pub fn clip_add(&self, kind: &str, text: &str, app: &str, max_items: u32) -> Result<(), String> {
        let c = self.conn.lock().unwrap();
        let h = self.hash(kind, text);
        let blob = self.seal(text)?;
        // Duplicates are moved to the top (re-inserted with a new id) keeping their flags.
        let flags: Option<(bool, bool)> = c
            .query_row("SELECT pinned, favorite FROM clipboard_items WHERE content_hash = ?1", [&h], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()
            .map_err(|e| e.to_string())?;
        let (pinned, favorite) = flags.unwrap_or((false, false));
        c.execute("DELETE FROM clipboard_items WHERE content_hash = ?1", [&h]).map_err(|e| e.to_string())?;
        c.execute(
            "INSERT INTO clipboard_items(kind, content, content_hash, app, created_at, pinned, favorite) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![kind, blob, h, app, now(), pinned, favorite],
        )
        .map_err(|e| e.to_string())?;
        c.execute(
            "DELETE FROM clipboard_items WHERE pinned = 0 AND favorite = 0 AND id NOT IN
               (SELECT id FROM clipboard_items ORDER BY id DESC LIMIT ?1)",
            params![max_items],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clip_list(&self, query: &str, limit: u32) -> Result<Vec<ClipItem>, String> {
        self.cipher()?;
        let c = self.conn.lock().unwrap();
        let mut st = c
            .prepare("SELECT id, kind, content, app, created_at, pinned, favorite FROM clipboard_items ORDER BY pinned DESC, id DESC LIMIT 5000")
            .map_err(|e| e.to_string())?;
        let q = query.to_lowercase();
        let rows = st
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, Vec<u8>>(2)?, r.get::<_, String>(3)?, r.get::<_, i64>(4)?, r.get::<_, bool>(5)?, r.get::<_, bool>(6)?)))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows.flatten() {
            let Some(text) = self.open_blob(&row.2) else { continue };
            if !q.is_empty() && !text.to_lowercase().contains(&q) {
                continue;
            }
            out.push(ClipItem { id: row.0, kind: row.1, text, app: row.3, created_at: row.4, pinned: row.5, favorite: row.6 });
            if out.len() as u32 >= limit {
                break;
            }
        }
        Ok(out)
    }

    pub fn clip_get(&self, id: i64) -> Result<Option<String>, String> {
        let c = self.conn.lock().unwrap();
        let b: Option<Vec<u8>> = c.query_row("SELECT content FROM clipboard_items WHERE id = ?1", [id], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
        Ok(b.and_then(|b| self.open_blob(&b)))
    }

    pub fn clip_set_flag(&self, id: i64, flag: &str, value: bool) -> Result<(), String> {
        let col = match flag {
            "pinned" => "pinned",
            "favorite" => "favorite",
            _ => return Err("unknown flag".into()),
        };
        self.conn.lock().unwrap().execute(&format!("UPDATE clipboard_items SET {col} = ?1 WHERE id = ?2"), params![value, id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clip_delete(&self, id: i64) -> Result<(), String> {
        self.conn.lock().unwrap().execute("DELETE FROM clipboard_items WHERE id = ?1", [id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clip_clear(&self, keep_pinned: bool) -> Result<(), String> {
        let c = self.conn.lock().unwrap();
        let sql = if keep_pinned { "DELETE FROM clipboard_items WHERE pinned = 0 AND favorite = 0" } else { "DELETE FROM clipboard_items" };
        c.execute(sql, []).map_err(|e| e.to_string())?;
        c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;").map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clip_expire(&self, retention_days: u32) -> Result<usize, String> {
        let cutoff = now() - retention_days as i64 * 86400;
        self.conn
            .lock()
            .unwrap()
            .execute("DELETE FROM clipboard_items WHERE pinned = 0 AND favorite = 0 AND created_at < ?1", [cutoff])
            .map_err(|e| e.to_string())
    }

    pub fn clip_count(&self) -> i64 {
        self.conn.lock().unwrap().query_row("SELECT COUNT(*) FROM clipboard_items", [], |r| r.get(0)).unwrap_or(0)
    }

    // ---- snippets ----

    pub fn snippets(&self) -> Result<Vec<Snippet>, String> {
        let c = self.conn.lock().unwrap();
        let mut st = c.prepare("SELECT data FROM snippets ORDER BY updated_at DESC").map_err(|e| e.to_string())?;
        let rows = st.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
        Ok(rows.flatten().filter_map(|s| serde_json::from_str(&s).ok()).collect())
    }

    pub fn snippet_upsert(&self, s: &Snippet) -> Result<(), String> {
        let data = serde_json::to_string(s).map_err(|e| e.to_string())?;
        self.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO snippets(id, data, updated_at) VALUES (?1, ?2, ?3) ON CONFLICT(id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at",
                params![s.id, data, now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn snippet_delete(&self, id: &str) -> Result<(), String> {
        self.conn.lock().unwrap().execute("DELETE FROM snippets WHERE id = ?1", [id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn counters(&self) -> std::collections::HashMap<String, i64> {
        let c = self.conn.lock().unwrap();
        let mut st = c.prepare("SELECT name, value FROM counters").unwrap();
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))).unwrap();
        rows.flatten().collect()
    }

    pub fn save_counters(&self, m: &std::collections::HashMap<String, i64>) {
        let c = self.conn.lock().unwrap();
        for (k, v) in m {
            let _ = c.execute("INSERT INTO counters(name, value) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET value = excluded.value", params![k, v]);
        }
    }

    // ---- usage (ids and counts only — never content) ----

    pub fn record_usage(&self, action_id: &str) {
        let _ = self.conn.lock().unwrap().execute(
            "INSERT INTO action_usage(action_id, count, last_used) VALUES (?1, 1, ?2) ON CONFLICT(action_id) DO UPDATE SET count = count + 1, last_used = excluded.last_used",
            params![action_id, now()],
        );
    }

    pub fn usage(&self, limit: u32) -> Vec<(String, i64, i64)> {
        let c = self.conn.lock().unwrap();
        let mut st = c.prepare("SELECT action_id, count, last_used FROM action_usage ORDER BY last_used DESC LIMIT ?1").unwrap();
        let rows = st.query_map([limit], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
        rows.flatten().collect()
    }

    pub fn clear_usage(&self) -> Result<(), String> {
        self.conn.lock().unwrap().execute("DELETE FROM action_usage", []).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_dedupe_limits_and_encryption() {
        let db = Db::open_in_memory();
        for i in 0..15 {
            db.clip_add("text", &format!("item {i}"), "app", 10).unwrap();
        }
        db.clip_add("text", "item 14", "app", 10).unwrap();
        let items = db.clip_list("", 100).unwrap();
        assert_eq!(items.len(), 10);
        assert_eq!(items[0].text, "item 14");
        assert_eq!(db.clip_list("item 1", 100).unwrap().len(), 5); // 10..=14
        // Content is not stored in plaintext.
        let raw: Vec<u8> = db.conn.lock().unwrap().query_row("SELECT content FROM clipboard_items LIMIT 1", [], |r| r.get(0)).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("item"));
        let id = items[3].id;
        db.clip_set_flag(id, "pinned", true).unwrap();
        db.clip_clear(true).unwrap();
        assert_eq!(db.clip_count(), 1);
        db.clip_clear(false).unwrap();
        assert_eq!(db.clip_count(), 0);
    }

    #[test]
    fn snippets_and_usage() {
        let db = Db::open_in_memory();
        let s = Snippet { id: "a".into(), name: "A".into(), trigger: ";a".into(), content: "x".into(), category: "".into(), case_sensitive: false, favorite: false, enabled: true, apps: vec![] };
        db.snippet_upsert(&s).unwrap();
        db.snippet_upsert(&Snippet { content: "y".into(), ..s.clone() }).unwrap();
        assert_eq!(db.snippets().unwrap()[0].content, "y");
        db.snippet_delete("a").unwrap();
        assert!(db.snippets().unwrap().is_empty());
        db.record_usage("ai.polish");
        db.record_usage("ai.polish");
        assert_eq!(db.usage(5)[0].1, 2);
    }
}
