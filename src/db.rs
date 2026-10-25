//! SQLite connection and schema. The schema matches the earlier Python version exactly,
//! so an existing database file keeps working.

use rusqlite::Connection;

const SCHEMA_VERSION: i64 = 1;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS users (
    id            INTEGER PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS sessions (
    token_hash TEXT PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS classes (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    section    TEXT NOT NULL DEFAULT '',
    color      TEXT NOT NULL DEFAULT 'oxblood',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- A to-do. When class_id is set it is also an assignment in that class.
CREATE TABLE IF NOT EXISTS todos (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    class_id     INTEGER REFERENCES classes(id) ON DELETE SET NULL,
    title        TEXT NOT NULL,
    due_date     TEXT,
    priority     INTEGER NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 3),
    done         INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- Documents. Each page belongs to either a to-do or a class (its central document).
CREATE TABLE IF NOT EXISTS pages (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    todo_id    INTEGER REFERENCES todos(id) ON DELETE CASCADE,
    class_id   INTEGER REFERENCES classes(id) ON DELETE CASCADE,
    title      TEXT NOT NULL DEFAULT 'Untitled',
    body       TEXT NOT NULL DEFAULT '',
    position   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK ((todo_id IS NULL) != (class_id IS NULL))
);

CREATE INDEX IF NOT EXISTS idx_todos_user ON todos(user_id, class_id);
CREATE INDEX IF NOT EXISTS idx_classes_user ON classes(user_id);
CREATE INDEX IF NOT EXISTS idx_pages_todo ON pages(todo_id, position);
CREATE INDEX IF NOT EXISTS idx_pages_class ON pages(class_id, position);
";

/// Opens (creating if needed) the database and applies the schema.
pub fn open(path: &str) -> rusqlite::Result<Connection> {
    if let Some(dir) = std::path::Path::new(path).parent() {
        if !dir.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(dir);
        }
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    conn.execute_batch(
        "PRAGMA synchronous = NORMAL;
         PRAGMA cache_size = -512;", // ~512 KB page cache: plenty for one user, easy on RAM
    )?;
    conn.execute_batch(SCHEMA)?;
    conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
    Ok(conn)
}
