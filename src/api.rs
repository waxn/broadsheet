//! JSON API handlers. A port of the Python version with identical routes, validation,
//! error messages and SQL. Every query is scoped to the signed-in user.

use std::collections::HashMap;
use std::sync::{MutexGuard, OnceLock};

use rusqlite::types::{Value as Sql, ValueRef};
use rusqlite::{params, params_from_iter, Connection, ErrorCode};
use serde_json::{json, Map, Value};

use crate::{auth, State};

const CLASS_COLORS: [&str; 6] = ["oxblood", "forest", "navy", "ochre", "plum", "slate"];
const MAX_BODY_CHARS: usize = 200_000;

// ---------- request / reply types ----------

pub struct ApiError {
    pub status: u16,
    pub message: String,
}

impl ApiError {
    fn new(status: u16, message: impl Into<String>) -> ApiError {
        ApiError { status, message: message.into() }
    }
}

fn bad(message: impl Into<String>) -> ApiError {
    ApiError::new(400, message)
}

impl From<rusqlite::Error> for ApiError {
    fn from(err: rusqlite::Error) -> ApiError {
        eprintln!("database error: {err}");
        ApiError::new(500, "Something broke on the server")
    }
}

type ApiResult<T> = Result<T, ApiError>;

pub struct Reply {
    pub status: u16,
    pub payload: Value,
    pub cookies: Vec<String>,
}

impl Reply {
    fn new(status: u16, payload: Value) -> Reply {
        Reply { status, payload, cookies: Vec::new() }
    }
}

fn ok(payload: Value) -> ApiResult<Reply> {
    Ok(Reply::new(200, payload))
}

fn created(payload: Value) -> ApiResult<Reply> {
    Ok(Reply::new(201, payload))
}

pub fn error_reply(err: ApiError) -> Reply {
    Reply::new(err.status, json!({ "error": err.message }))
}

pub struct User {
    pub id: i64,
    pub username: String,
}

pub struct Ctx<'a> {
    pub state: &'a State,
    pub user: Option<User>,
    pub body: Option<Map<String, Value>>,
    pub query: HashMap<String, String>,
    pub params: Vec<i64>,
    pub token: Option<String>,
}

impl Ctx<'_> {
    fn db(&self) -> MutexGuard<'_, Connection> {
        self.state.db.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn uid(&self) -> ApiResult<i64> {
        self.user.as_ref().map(|u| u.id).ok_or_else(|| ApiError::new(401, "Sign in first"))
    }

    fn body(&self) -> ApiResult<&Map<String, Value>> {
        self.body.as_ref().ok_or_else(|| bad("Expected a JSON object"))
    }
}

// ---------- database helpers ----------

fn value_json(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null | ValueRef::Blob(_) => Value::Null,
        ValueRef::Integer(n) => json!(n),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).into_owned()),
    }
}

/// Runs a query and returns each row as a JSON object keyed by column name.
fn rows(conn: &Connection, sql: &str, p: impl rusqlite::Params) -> rusqlite::Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let names: Vec<String> = stmt.column_names().map(|s| s.to_string()).collect();
    let mapped = stmt.query_map(p, |row| {
        let mut object = Map::new();
        for (i, name) in names.iter().enumerate() {
            object.insert(name.clone(), value_json(row.get_ref(i)?));
        }
        Ok(Value::Object(object))
    })?;
    mapped.collect()
}

fn one(conn: &Connection, sql: &str, p: impl rusqlite::Params) -> rusqlite::Result<Option<Value>> {
    Ok(rows(conn, sql, p)?.into_iter().next())
}

fn exists(conn: &Connection, sql: &str, p: impl rusqlite::Params) -> rusqlite::Result<bool> {
    conn.prepare(sql)?.exists(p)
}

fn now_sql(conn: &Connection) -> rusqlite::Result<String> {
    conn.query_row("SELECT datetime('now')", [], |r| r.get(0))
}

// ---------- validation helpers ----------

/// Python-style truthiness, used where the old API accepted any JSON value for a flag.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn clean_text(v: Option<&Value>, field: &str, max_len: usize, required: bool) -> ApiResult<String> {
    let text = match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.trim().to_string(),
        Some(_) => return Err(bad(format!("{field} must be text"))),
    };
    if required && text.is_empty() {
        return Err(bad(format!("{field} is required")));
    }
    if text.chars().count() > max_len {
        return Err(bad(format!("{field} is too long")));
    }
    Ok(text)
}

fn is_leap(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    if !(digits(0..4) && digits(5..7) && digits(8..10)) {
        return false;
    }
    let year: u32 = s[0..4].parse().unwrap_or(0);
    let month: u32 = s[5..7].parse().unwrap_or(0);
    let day: u32 = s[8..10].parse().unwrap_or(0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => return false,
    };
    year >= 1 && (1..=days_in_month).contains(&day)
}

fn clean_date(v: Option<&Value>) -> ApiResult<Option<String>> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(Value::String(s)) if valid_date(s) => Ok(Some(s.clone())),
        Some(_) => Err(bad("due_date must be YYYY-MM-DD")),
    }
}

fn clean_priority(v: Option<&Value>) -> ApiResult<i64> {
    match v.and_then(Value::as_i64) {
        Some(n) if (0..=3).contains(&n) => Ok(n),
        _ => Err(bad("priority must be 0 (none) to 3 (high)")),
    }
}

fn clean_body(v: Option<&Value>) -> ApiResult<String> {
    match v {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) if s.chars().count() > MAX_BODY_CHARS => Err(bad("document is too long")),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(bad("body must be text")),
    }
}

fn clean_color(v: Option<&Value>) -> ApiResult<String> {
    match v {
        None | Some(Value::Null) => Ok("oxblood".into()),
        Some(Value::String(s)) if s.is_empty() => Ok("oxblood".into()),
        Some(Value::String(s)) if CLASS_COLORS.contains(&s.as_str()) => Ok(s.clone()),
        Some(_) => Err(bad("Unknown color")),
    }
}

/// An optional integer id: absent, null or "" mean none.
fn opt_id(v: Option<&Value>, field: &str) -> ApiResult<Option<i64>> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(Value::Number(n)) if n.is_i64() => Ok(n.as_i64()),
        Some(_) => Err(bad(format!("{field} must be an integer"))),
    }
}

fn owned_class(conn: &Connection, uid: i64, v: Option<&Value>) -> ApiResult<Option<i64>> {
    let Some(id) = opt_id(v, "class_id")? else { return Ok(None) };
    if exists(conn, "SELECT 1 FROM classes WHERE id = ?1 AND user_id = ?2", params![id, uid])? {
        Ok(Some(id))
    } else {
        Err(ApiError::new(404, "Class not found"))
    }
}

fn require_todo(conn: &Connection, uid: i64, id: i64) -> ApiResult<()> {
    if exists(conn, "SELECT 1 FROM todos WHERE id = ?1 AND user_id = ?2", params![id, uid])? {
        Ok(())
    } else {
        Err(ApiError::new(404, "To-do not found"))
    }
}

fn require_class(conn: &Connection, uid: i64, id: i64) -> ApiResult<Value> {
    one(conn, "SELECT * FROM classes WHERE id = ?1 AND user_id = ?2", params![id, uid])?
        .ok_or_else(|| ApiError::new(404, "Class not found"))
}

fn require_page(conn: &Connection, uid: i64, id: i64) -> ApiResult<Value> {
    one(conn, "SELECT * FROM pages WHERE id = ?1 AND user_id = ?2", params![id, uid])?
        .ok_or_else(|| ApiError::new(404, "Page not found"))
}

fn insert_default_page(conn: &Connection, uid: i64, todo_id: Option<i64>, class_id: Option<i64>) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO pages (user_id, todo_id, class_id, title, position) VALUES (?1, ?2, ?3, 'Notes', 0)",
        params![uid, todo_id, class_id],
    )?;
    Ok(())
}

// ---------- auth ----------

fn session_cookie(token: &str, max_age: i64, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}", auth::SESSION_COOKIE)
}

fn start_session(ctx: &Ctx, conn: &Connection, user_id: i64) -> ApiResult<String> {
    let days = ctx.state.cfg.session_days;
    let token = auth::new_session_token();
    conn.execute("DELETE FROM sessions WHERE expires_at < strftime('%s','now')", [])?;
    conn.execute(
        "INSERT INTO sessions (token_hash, user_id, expires_at) VALUES (?1, ?2, ?3)",
        params![auth::hash_token(&token), user_id, auth::unix_now() + days * 86_400],
    )?;
    Ok(session_cookie(&token, days * 86_400, ctx.state.cfg.cookie_secure))
}

fn valid_username(name: &str) -> bool {
    (3..=32).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn signup(ctx: &Ctx) -> ApiResult<Reply> {
    if !ctx.state.cfg.allow_signup {
        return Err(ApiError::new(403, "Sign-up is closed on this server"));
    }
    let body = ctx.body()?;
    let username = match body.get("username") {
        Some(Value::String(s)) if valid_username(s) => s.clone(),
        _ => return Err(bad("Username must be 3–32 letters, digits or underscores")),
    };
    let password = match body.get("password") {
        Some(Value::String(s)) if (8..=200).contains(&s.chars().count()) => s.clone(),
        _ => return Err(bad("Password must be 8–200 characters")),
    };
    // Hash before taking the database lock: it is the slow part and must not block other requests.
    let hash = auth::hash_password(&password);

    let db = ctx.db();
    let inserted = db.execute("INSERT INTO users (username, password_hash) VALUES (?1, ?2)", params![username, hash]);
    match inserted {
        Ok(_) => {}
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == ErrorCode::ConstraintViolation => {
            return Err(ApiError::new(409, "That username is taken"));
        }
        Err(e) => return Err(e.into()),
    }
    let id = db.last_insert_rowid();
    let cookie = start_session(ctx, &db, id)?;
    let mut reply = Reply::new(201, json!({ "id": id, "username": username }));
    reply.cookies.push(cookie);
    Ok(reply)
}

/// What the old API did with `body.get(key) or ""`, then `str(...)`.
fn text_or_empty(v: Option<&Value>) -> String {
    match v {
        None => String::new(),
        Some(x) if !truthy(x) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

fn login(ctx: &Ctx) -> ApiResult<Reply> {
    let body = ctx.body()?;
    let username = text_or_empty(body.get("username"));
    let password = text_or_empty(body.get("password"));

    let found = {
        let db = ctx.db();
        db.query_row(
            "SELECT id, username, password_hash FROM users WHERE username = ?1",
            params![username],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)),
        )
        .ok()
    };
    // Hash outside the lock. An unknown user still costs one hash, so timing does not reveal accounts.
    static DUMMY: OnceLock<String> = OnceLock::new();
    let stored = found.as_ref().map(|f| f.2.clone()).unwrap_or_else(|| DUMMY.get_or_init(|| auth::hash_password("dummy")).clone());
    let verified = auth::verify_password(&password, &stored);

    let (id, name) = match found {
        Some((id, name, _)) if verified => (id, name),
        _ => return Err(ApiError::new(401, "Wrong username or password")),
    };
    let db = ctx.db();
    let cookie = start_session(ctx, &db, id)?;
    let mut reply = Reply::new(200, json!({ "id": id, "username": name }));
    reply.cookies.push(cookie);
    Ok(reply)
}

fn logout(ctx: &Ctx) -> ApiResult<Reply> {
    if let Some(token) = &ctx.token {
        ctx.db().execute("DELETE FROM sessions WHERE token_hash = ?1", params![auth::hash_token(token)])?;
    }
    let mut reply = Reply::new(200, json!({ "ok": true }));
    reply.cookies.push(format!("{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0", auth::SESSION_COOKIE));
    Ok(reply)
}

fn me(ctx: &Ctx) -> ApiResult<Reply> {
    let user = ctx.user.as_ref().ok_or_else(|| ApiError::new(401, "Sign in first"))?;
    ok(json!({ "id": user.id, "username": user.username }))
}

// ---------- search ----------

fn search(ctx: &Ctx) -> ApiResult<Reply> {
    let q: String = ctx.query.get("q").map(|s| s.trim().chars().take(80).collect()).unwrap_or_default();
    if q.is_empty() {
        return ok(json!({ "todos": [], "classes": [], "pages": [] }));
    }
    let like = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let uid = ctx.uid()?;
    let db = ctx.db();
    let todos = rows(
        &db,
        &format!("{TODO_SELECT} WHERE t.user_id = ?1 AND t.title LIKE ?2 ESCAPE '\\' ORDER BY t.done, t.id DESC LIMIT 8"),
        params![uid, like],
    )?;
    let classes = rows(
        &db,
        "SELECT id, name, section, color FROM classes
         WHERE user_id = ?1 AND (name LIKE ?2 ESCAPE '\\' OR section LIKE ?2 ESCAPE '\\') LIMIT 6",
        params![uid, like],
    )?;
    let pages = rows(
        &db,
        "SELECT p.id, p.title, p.todo_id, p.class_id, COALESCE(t.title, c.name) AS owner_name
         FROM pages p LEFT JOIN todos t ON t.id = p.todo_id LEFT JOIN classes c ON c.id = p.class_id
         WHERE p.user_id = ?1 AND (p.title LIKE ?2 ESCAPE '\\' OR p.body LIKE ?2 ESCAPE '\\')
         ORDER BY p.updated_at DESC LIMIT 8",
        params![uid, like],
    )?;
    ok(json!({ "todos": todos, "classes": classes, "pages": pages }))
}

// ---------- to-dos ----------

const TODO_SELECT: &str = "SELECT t.*, c.name AS class_name, c.color AS class_color
    FROM todos t LEFT JOIN classes c ON c.id = t.class_id";

fn todo_row(conn: &Connection, id: i64) -> ApiResult<Value> {
    one(conn, &format!("{TODO_SELECT} WHERE t.id = ?1"), params![id])?.ok_or_else(|| ApiError::new(404, "To-do not found"))
}

fn todos_list(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let list = rows(
        &ctx.db(),
        &format!("{TODO_SELECT} WHERE t.user_id = ?1 ORDER BY t.done, t.due_date IS NULL, t.due_date, t.priority DESC, t.id"),
        params![uid],
    )?;
    ok(Value::Array(list))
}

fn todo_create(ctx: &Ctx) -> ApiResult<Reply> {
    let body = ctx.body()?;
    let uid = ctx.uid()?;
    let title = clean_text(body.get("title"), "title", 300, true)?;
    let db = ctx.db();
    let class_id = owned_class(&db, uid, body.get("class_id"))?;
    let due = clean_date(body.get("due_date"))?;
    let priority = match body.get("priority") {
        None => 0,
        some => clean_priority(some)?,
    };
    let tx = db.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO todos (user_id, class_id, title, due_date, priority) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![uid, class_id, title, due, priority],
    )?;
    let id = tx.last_insert_rowid();
    insert_default_page(&tx, uid, Some(id), None)?;
    tx.commit()?;
    created(todo_row(&db, id)?)
}

fn todo_get(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let db = ctx.db();
    require_todo(&db, uid, ctx.params[0])?;
    ok(todo_row(&db, ctx.params[0])?)
}

fn todo_update(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    require_todo(&db, uid, id)?;
    let body = ctx.body()?;

    let mut sets: Vec<&str> = Vec::new();
    let mut values: Vec<Sql> = Vec::new();
    if let Some(v) = body.get("title") {
        sets.push("title = ?");
        values.push(Sql::Text(clean_text(Some(v), "title", 300, true)?));
    }
    if body.contains_key("due_date") {
        sets.push("due_date = ?");
        values.push(clean_date(body.get("due_date"))?.map_or(Sql::Null, Sql::Text));
    }
    if body.contains_key("priority") {
        sets.push("priority = ?");
        values.push(Sql::Integer(clean_priority(body.get("priority"))?));
    }
    if body.contains_key("class_id") {
        sets.push("class_id = ?");
        values.push(owned_class(&db, uid, body.get("class_id"))?.map_or(Sql::Null, Sql::Integer));
    }
    if let Some(v) = body.get("done") {
        let done = truthy(v);
        sets.push("done = ?");
        values.push(Sql::Integer(i64::from(done)));
        sets.push("completed_at = ?");
        values.push(if done { Sql::Text(now_sql(&db)?) } else { Sql::Null });
    }
    if !sets.is_empty() {
        sets.push("updated_at = datetime('now')");
        values.push(Sql::Integer(id));
        db.execute(&format!("UPDATE todos SET {} WHERE id = ?", sets.join(", ")), params_from_iter(values))?;
    }
    ok(todo_row(&db, id)?)
}

fn todo_delete(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let db = ctx.db();
    require_todo(&db, uid, ctx.params[0])?;
    db.execute("DELETE FROM todos WHERE id = ?1", params![ctx.params[0]])?;
    ok(json!({ "ok": true }))
}

// ---------- classes ----------

fn classes_list(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let list = rows(
        &ctx.db(),
        "SELECT c.*,
                COUNT(t.id) AS task_count,
                COALESCE(SUM(CASE WHEN t.done = 0 THEN 1 ELSE 0 END), 0) AS open_count
         FROM classes c LEFT JOIN todos t ON t.class_id = c.id
         WHERE c.user_id = ?1
         GROUP BY c.id
         ORDER BY lower(c.name)",
        params![uid],
    )?;
    ok(Value::Array(list))
}

fn class_create(ctx: &Ctx) -> ApiResult<Reply> {
    let body = ctx.body()?;
    let uid = ctx.uid()?;
    let name = clean_text(body.get("name"), "name", 120, true)?;
    let section = clean_text(body.get("section"), "section", 120, false)?;
    let color = clean_color(body.get("color"))?;
    let db = ctx.db();
    let tx = db.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO classes (user_id, name, section, color) VALUES (?1, ?2, ?3, ?4)",
        params![uid, name, section, color],
    )?;
    let id = tx.last_insert_rowid();
    insert_default_page(&tx, uid, None, Some(id))?;
    tx.commit()?;
    created(require_class(&db, uid, id)?)
}

fn class_get(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    let class = require_class(&db, uid, id)?;
    let todos = rows(
        &db,
        &format!("{TODO_SELECT} WHERE t.user_id = ?1 AND t.class_id = ?2 ORDER BY t.done, t.due_date IS NULL, t.due_date, t.id"),
        params![uid, id],
    )?;
    ok(json!({ "class": class, "todos": todos }))
}

fn class_update(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    require_class(&db, uid, id)?;
    let body = ctx.body()?;

    let mut sets: Vec<&str> = Vec::new();
    let mut values: Vec<Sql> = Vec::new();
    if let Some(v) = body.get("name") {
        sets.push("name = ?");
        values.push(Sql::Text(clean_text(Some(v), "name", 120, true)?));
    }
    if let Some(v) = body.get("section") {
        sets.push("section = ?");
        values.push(Sql::Text(clean_text(Some(v), "section", 120, false)?));
    }
    if body.contains_key("color") {
        sets.push("color = ?");
        values.push(Sql::Text(clean_color(body.get("color"))?));
    }
    if !sets.is_empty() {
        sets.push("updated_at = datetime('now')");
        values.push(Sql::Integer(id));
        db.execute(&format!("UPDATE classes SET {} WHERE id = ?", sets.join(", ")), params_from_iter(values))?;
    }
    ok(require_class(&db, uid, id)?)
}

fn class_delete(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let db = ctx.db();
    require_class(&db, uid, ctx.params[0])?;
    db.execute("DELETE FROM classes WHERE id = ?1", params![ctx.params[0]])?;
    ok(json!({ "ok": true }))
}

// ---------- pages (documents) ----------

fn query_id(ctx: &Ctx, key: &str) -> ApiResult<Option<i64>> {
    match ctx.query.get(key).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => s.parse().map(Some).map_err(|_| bad(format!("{key} must be an integer"))),
    }
}

fn pages_list(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let todo_id = query_id(ctx, "todo_id")?;
    let class_id = query_id(ctx, "class_id")?;
    if todo_id.is_some() == class_id.is_some() {
        return Err(bad("Pass exactly one of todo_id or class_id"));
    }
    let db = ctx.db();
    let list = if let Some(id) = todo_id {
        require_todo(&db, uid, id)?;
        rows(&db, "SELECT * FROM pages WHERE user_id = ?1 AND todo_id = ?2 ORDER BY position, id", params![uid, id])?
    } else {
        let id = class_id.unwrap_or_default();
        require_class(&db, uid, id)?;
        rows(&db, "SELECT * FROM pages WHERE user_id = ?1 AND class_id = ?2 ORDER BY position, id", params![uid, id])?
    };
    ok(Value::Array(list))
}

fn page_create(ctx: &Ctx) -> ApiResult<Reply> {
    let body = ctx.body()?;
    let uid = ctx.uid()?;
    let title_value = body.get("title").filter(|v| truthy(v)).cloned().unwrap_or_else(|| json!("Untitled"));
    let title = clean_text(Some(&title_value), "title", 120, true)?;
    let todo_id = opt_id(body.get("todo_id"), "todo_id")?;
    let class_id = opt_id(body.get("class_id"), "class_id")?;
    if todo_id.is_some() == class_id.is_some() {
        return Err(bad("Pass exactly one of todo_id or class_id"));
    }
    let db = ctx.db();
    let new_id = {
        let tx = db.unchecked_transaction()?;
        let position: i64 = if let Some(id) = todo_id {
            require_todo(&tx, uid, id)?;
            tx.query_row("SELECT COALESCE(MAX(position), -1) + 1 FROM pages WHERE todo_id = ?1", params![id], |r| r.get(0))?
        } else {
            let id = class_id.unwrap_or_default();
            require_class(&tx, uid, id)?;
            tx.query_row("SELECT COALESCE(MAX(position), -1) + 1 FROM pages WHERE class_id = ?1", params![id], |r| r.get(0))?
        };
        tx.execute(
            "INSERT INTO pages (user_id, todo_id, class_id, title, position) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![uid, todo_id, class_id, title, position],
        )?;
        let new_id = tx.last_insert_rowid();
        tx.commit()?;
        new_id
    };
    created(require_page(&db, uid, new_id)?)
}

fn page_get(ctx: &Ctx) -> ApiResult<Reply> {
    ok(require_page(&ctx.db(), ctx.uid()?, ctx.params[0])?)
}

fn page_update(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    require_page(&db, uid, id)?;
    let body = ctx.body()?;

    let mut sets: Vec<&str> = Vec::new();
    let mut values: Vec<Sql> = Vec::new();
    if let Some(v) = body.get("title") {
        sets.push("title = ?");
        values.push(Sql::Text(clean_text(Some(v), "title", 120, true)?));
    }
    if body.contains_key("body") {
        sets.push("body = ?");
        values.push(Sql::Text(clean_body(body.get("body"))?));
    }
    if !sets.is_empty() {
        sets.push("updated_at = datetime('now')");
        values.push(Sql::Integer(id));
        db.execute(&format!("UPDATE pages SET {} WHERE id = ?", sets.join(", ")), params_from_iter(values))?;
    }
    ok(require_page(&db, uid, id)?)
}

fn page_delete(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let db = ctx.db();
    require_page(&db, uid, ctx.params[0])?;
    db.execute("DELETE FROM pages WHERE id = ?1", params![ctx.params[0]])?;
    ok(json!({ "ok": true }))
}

// ---------- routing ----------

type Handler = fn(&Ctx) -> ApiResult<Reply>;

struct Route {
    method: &'static str,
    pattern: &'static str,
    handler: Handler,
    needs_auth: bool,
}

const fn route(method: &'static str, pattern: &'static str, handler: Handler, needs_auth: bool) -> Route {
    Route { method, pattern, handler, needs_auth }
}

static ROUTES: &[Route] = &[
    route("POST", "/api/signup", signup, false),
    route("POST", "/api/login", login, false),
    route("POST", "/api/logout", logout, false),
    route("GET", "/api/me", me, true),
    route("GET", "/api/search", search, true),
    route("GET", "/api/todos", todos_list, true),
    route("POST", "/api/todos", todo_create, true),
    route("GET", "/api/todos/:id", todo_get, true),
    route("PATCH", "/api/todos/:id", todo_update, true),
    route("DELETE", "/api/todos/:id", todo_delete, true),
    route("GET", "/api/classes", classes_list, true),
    route("POST", "/api/classes", class_create, true),
    route("GET", "/api/classes/:id", class_get, true),
    route("PATCH", "/api/classes/:id", class_update, true),
    route("DELETE", "/api/classes/:id", class_delete, true),
    route("GET", "/api/pages", pages_list, true),
    route("POST", "/api/pages", page_create, true),
    route("GET", "/api/pages/:id", page_get, true),
    route("PATCH", "/api/pages/:id", page_update, true),
    route("DELETE", "/api/pages/:id", page_delete, true),
];

/// Matches "/api/todos/:id" style patterns; ":id" segments are all-digit numbers.
fn match_pattern(pattern: &str, path: &str) -> Option<Vec<i64>> {
    let mut ids = Vec::new();
    let mut want = pattern.split('/');
    let mut have = path.split('/');
    loop {
        match (want.next(), have.next()) {
            (None, None) => return Some(ids),
            (Some(":id"), Some(seg)) if !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit()) => {
                ids.push(seg.parse().ok()?);
            }
            (Some(a), Some(b)) if a == b => {}
            _ => return None,
        }
    }
}

fn parse_cookies(header: Option<&String>) -> HashMap<String, String> {
    let mut jar = HashMap::new();
    for part in header.map(String::as_str).unwrap_or("").split(';') {
        if let Some((name, value)) = part.trim().split_once('=') {
            if !name.is_empty() {
                jar.insert(name.to_string(), value.to_string());
            }
        }
    }
    jar
}

/// Decodes one query value (percent escapes and '+').
fn unquote_plus(text: &str) -> String {
    let raw = text.replace('+', " ").into_bytes();
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%' && i + 3 <= raw.len() {
            if let Ok(byte) = u8::from_str_radix(&String::from_utf8_lossy(&raw[i + 1..i + 3]), 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(raw[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_query(raw: &str) -> HashMap<String, String> {
    raw.split('&')
        .filter_map(|part| {
            let (name, value) = part.split_once('=').unwrap_or((part, ""));
            (!name.is_empty()).then(|| (name.to_string(), unquote_plus(value)))
        })
        .collect()
}

pub struct ApiRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub query: &'a str,
    pub headers: &'a HashMap<String, String>,
    /// None means the body was over the size limit and was not read.
    pub body: Option<&'a [u8]>,
}

pub fn handle(state: &State, req: &ApiRequest) -> Reply {
    let mut found = None;
    let mut path_exists = false;
    for r in ROUTES {
        if let Some(ids) = match_pattern(r.pattern, req.path) {
            path_exists = true;
            if r.method == req.method {
                found = Some((r, ids));
                break;
            }
        }
    }
    let Some((route, params)) = found else {
        let (status, text) = if path_exists { (405, "Method Not Allowed") } else { (404, "Not Found") };
        return error_reply(ApiError::new(status, text));
    };
    let Some(raw_body) = req.body else {
        return error_reply(ApiError::new(413, "Request is too large"));
    };

    let token = parse_cookies(req.headers.get("cookie")).remove(auth::SESSION_COOKIE);
    let user = token.as_deref().and_then(|t| {
        let db = state.db.lock().unwrap_or_else(|p| p.into_inner());
        db.query_row(
            "SELECT u.id, u.username FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token_hash = ?1 AND s.expires_at > ?2",
            params![auth::hash_token(t), auth::unix_now()],
            |r| Ok(User { id: r.get(0)?, username: r.get(1)? }),
        )
        .ok()
    });
    if route.needs_auth && user.is_none() {
        return error_reply(ApiError::new(401, "Sign in first"));
    }

    let mut body = None;
    if matches!(req.method, "POST" | "PATCH" | "PUT" | "DELETE") && !raw_body.is_empty() {
        let is_json = req.headers.get("content-type").is_some_and(|c| c.contains("application/json"));
        match serde_json::from_slice::<Value>(raw_body) {
            Ok(Value::Object(map)) if is_json => body = Some(map),
            _ => return error_reply(bad("Body must be a JSON object")),
        }
    }

    let ctx = Ctx { state, user, body, query: parse_query(req.query), params, token };
    match (route.handler)(&ctx) {
        Ok(reply) => reply,
        Err(err) => error_reply(err),
    }
}
