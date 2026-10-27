//! SQLite connection and schema. Version 1 was the original tasks/classes/pages schema (shared
//! with the earlier Python version); version 2 adds sections, subtasks, scheduling, nested pages,
//! calendar events, habits and uploaded files. A version 1 database is upgraded in place.

use rusqlite::Connection;

const SCHEMA_VERSION: i64 = 2;

/// Tables that exist unchanged since version 1.
const BASE: &str = "
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
";

const V2: &str = "
CREATE TABLE IF NOT EXISTS classes (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    section    TEXT NOT NULL DEFAULT '',
    color      TEXT NOT NULL DEFAULT 'charcoal',
    emoji      TEXT NOT NULL DEFAULT '',
    position   INTEGER NOT NULL DEFAULT 0,
    archived   INTEGER NOT NULL DEFAULT 0,
    view       TEXT NOT NULL DEFAULT 'list',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Groups of tasks inside a class (or the Inbox when class_id is NULL); the columns of a board.
CREATE TABLE IF NOT EXISTS sections (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    class_id   INTEGER REFERENCES classes(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    position   INTEGER NOT NULL DEFAULT 0,
    collapsed  INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
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
    completed_at TEXT,
    parent_id    INTEGER REFERENCES todos(id) ON DELETE CASCADE,
    section_id   INTEGER REFERENCES sections(id) ON DELETE SET NULL,
    description  TEXT NOT NULL DEFAULT '',
    due_time     TEXT,
    recurrence   TEXT,
    start_at     TEXT,
    end_at       TEXT,
    position     INTEGER NOT NULL DEFAULT 0
);

-- Documents. A page may belong to a to-do or a class, sit under another page, or stand alone.
CREATE TABLE IF NOT EXISTS pages (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_id  INTEGER REFERENCES pages(id) ON DELETE CASCADE,
    todo_id    INTEGER REFERENCES todos(id) ON DELETE CASCADE,
    class_id   INTEGER REFERENCES classes(id) ON DELETE CASCADE,
    title      TEXT NOT NULL DEFAULT '',
    icon       TEXT NOT NULL DEFAULT '',
    font       TEXT NOT NULL DEFAULT 'default',
    full_width INTEGER NOT NULL DEFAULT 0,
    favorite   INTEGER NOT NULL DEFAULT 0,
    body       TEXT NOT NULL DEFAULT '',
    content    TEXT,
    text       TEXT NOT NULL DEFAULT '',
    position   INTEGER NOT NULL DEFAULT 0,
    archived   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Mentions inside a page, rewritten on every save; read backwards they are backlinks.
CREATE TABLE IF NOT EXISTS links (
    user_id   INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    from_page INTEGER NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    to_kind   TEXT NOT NULL,
    to_id     INTEGER NOT NULL,
    PRIMARY KEY (from_page, to_kind, to_id)
);

-- Calendar events. All-day events use YYYY-MM-DD (end inclusive), timed ones YYYY-MM-DDTHH:MM.
CREATE TABLE IF NOT EXISTS events (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    class_id   INTEGER REFERENCES classes(id) ON DELETE SET NULL,
    title      TEXT NOT NULL,
    start_at   TEXT NOT NULL,
    end_at     TEXT NOT NULL,
    all_day    INTEGER NOT NULL DEFAULT 0,
    color      TEXT NOT NULL DEFAULT '',
    location   TEXT NOT NULL DEFAULT '',
    notes      TEXT NOT NULL DEFAULT '',
    countdown  INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS habits (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    emoji      TEXT NOT NULL DEFAULT '',
    color      TEXT NOT NULL DEFAULT 'green',
    position   INTEGER NOT NULL DEFAULT 0,
    archived   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS habit_checks (
    habit_id INTEGER NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
    day      TEXT NOT NULL,
    PRIMARY KEY (habit_id, day)
);

-- Uploaded files; the bytes live in <data dir>/files/<sha256>.
CREATE TABLE IF NOT EXISTS files (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    mime       TEXT NOT NULL,
    size       INTEGER NOT NULL,
    sha256     TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_todos_user ON todos(user_id, class_id);
CREATE INDEX IF NOT EXISTS idx_todos_parent ON todos(parent_id);
CREATE INDEX IF NOT EXISTS idx_classes_user ON classes(user_id);
CREATE INDEX IF NOT EXISTS idx_sections_class ON sections(user_id, class_id);
CREATE INDEX IF NOT EXISTS idx_pages_user ON pages(user_id, parent_id);
CREATE INDEX IF NOT EXISTS idx_pages_todo ON pages(todo_id, position);
CREATE INDEX IF NOT EXISTS idx_pages_class ON pages(class_id, position);
CREATE INDEX IF NOT EXISTS idx_links_to ON links(user_id, to_kind, to_id);
CREATE INDEX IF NOT EXISTS idx_events_user ON events(user_id, start_at);
";

/// Upgrades a version 1 database. `pages` is rebuilt because its old CHECK required an owner.
const MIGRATE_1_TO_2: &str = "
ALTER TABLE classes ADD COLUMN emoji TEXT NOT NULL DEFAULT '';
ALTER TABLE classes ADD COLUMN position INTEGER NOT NULL DEFAULT 0;
ALTER TABLE classes ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE classes ADD COLUMN view TEXT NOT NULL DEFAULT 'list';
UPDATE classes SET color = CASE color
    WHEN 'oxblood' THEN 'berry_red' WHEN 'forest' THEN 'green' WHEN 'navy' THEN 'blue'
    WHEN 'ochre' THEN 'yellow' WHEN 'plum' THEN 'grape' ELSE 'charcoal' END;
UPDATE classes SET position = (SELECT COUNT(*) FROM classes c2
    WHERE c2.user_id = classes.user_id AND lower(c2.name) < lower(classes.name));

ALTER TABLE todos ADD COLUMN parent_id INTEGER REFERENCES todos(id) ON DELETE CASCADE;
ALTER TABLE todos ADD COLUMN section_id INTEGER REFERENCES sections(id) ON DELETE SET NULL;
ALTER TABLE todos ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE todos ADD COLUMN due_time TEXT;
ALTER TABLE todos ADD COLUMN recurrence TEXT;
ALTER TABLE todos ADD COLUMN start_at TEXT;
ALTER TABLE todos ADD COLUMN end_at TEXT;
ALTER TABLE todos ADD COLUMN position INTEGER NOT NULL DEFAULT 0;
UPDATE todos SET position = id;

ALTER TABLE pages RENAME TO pages_v1;
DROP INDEX IF EXISTS idx_pages_todo;
DROP INDEX IF EXISTS idx_pages_class;
";

const MIGRATE_PAGES: &str = "
INSERT INTO pages (id, user_id, todo_id, class_id, title, body, position, created_at, updated_at)
    SELECT id, user_id, todo_id, class_id, title, body, position, created_at, updated_at FROM pages_v1;
-- Empty default pages made for every to-do are not needed any more: tasks have descriptions.
DELETE FROM pages WHERE todo_id IS NOT NULL AND body = '' AND title = 'Notes';
DROP TABLE pages_v1;
";

fn user_version(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
}

fn has_table(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    conn.prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1")?.exists([name])
}

/// Opens (creating if needed) the database and brings the schema up to date.
pub fn open(path: &str) -> rusqlite::Result<Connection> {
    if let Some(dir) = std::path::Path::new(path).parent() {
        if !dir.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(dir);
        }
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    conn.execute_batch(
        "PRAGMA synchronous = NORMAL;
         PRAGMA cache_size = -512;", // ~512 KB page cache: plenty for one user, easy on RAM
    )?;
    conn.execute_batch(BASE)?;

    // Version 1 databases have todos but user_version 1 (or 0 if made before versioning).
    if user_version(&conn)? < 2 && has_table(&conn, "todos")? {
        // Foreign keys must be off while pages is rebuilt, or the copy would cascade.
        conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(MIGRATE_1_TO_2)?;
        tx.execute_batch(V2)?;
        tx.execute_batch(MIGRATE_PAGES)?;
        tx.commit()?;
    }
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(V2)?;
    conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
    Ok(conn)
}
