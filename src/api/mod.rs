//! JSON API: request/reply types, shared validation and database helpers, auth and routing.
//! The handlers live in one module per area. Every query is scoped to the signed-in user.

mod calendar;
mod classes;
mod files;
mod pages;
mod search;
mod tasks;

use std::collections::HashMap;
use std::sync::{MutexGuard, OnceLock};

use rusqlite::types::{Value as Sql, ValueRef};
use rusqlite::{params, params_from_iter, Connection, ErrorCode};
use serde_json::{json, Map, Value};

use crate::{auth, recur, State};

/// Todoist's palette, by name. The frontend owns the actual colours.
pub(crate) const COLORS: [&str; 20] = [
    "berry_red", "red", "orange", "yellow", "olive_green", "lime_green", "green", "mint_green", "teal", "sky_blue",
    "light_blue", "blue", "grape", "violet", "lavender", "magenta", "salmon", "charcoal", "grey", "taupe",
];

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

fn not_found(what: &str) -> ApiError {
    ApiError::new(404, format!("{what} not found"))
}

impl From<rusqlite::Error> for ApiError {
    fn from(err: rusqlite::Error) -> ApiError {
        eprintln!("database error: {err}");
        ApiError::new(500, "Something broke on the server")
    }
}

type ApiResult<T> = Result<T, ApiError>;

/// A non-JSON response body (an uploaded file).
pub struct FileBody {
    pub ctype: String,
    pub bytes: Vec<u8>,
    pub headers: Vec<(&'static str, String)>,
}

pub struct Reply {
    pub status: u16,
    pub payload: Value,
    pub cookies: Vec<String>,
    pub file: Option<FileBody>,
}

impl Reply {
    fn new(status: u16, payload: Value) -> Reply {
        Reply { status, payload, cookies: Vec::new(), file: None }
    }
}

fn ok(payload: Value) -> ApiResult<Reply> {
    Ok(Reply::new(200, payload))
}

fn created(payload: Value) -> ApiResult<Reply> {
    Ok(Reply::new(201, payload))
}

fn done() -> ApiResult<Reply> {
    ok(json!({ "ok": true }))
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
    /// The unparsed body, for routes that take raw bytes (uploads).
    pub raw: &'a [u8],
    pub headers: &'a HashMap<String, String>,
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

    fn query_id(&self, key: &str) -> ApiResult<Option<i64>> {
        match self.query.get(key).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some(s) => s.parse().map(Some).map_err(|_| bad(format!("{key} must be an integer"))),
        }
    }

    fn query_date(&self, key: &str) -> ApiResult<Option<String>> {
        match self.query.get(key).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some(s) if recur::parse_date(s).is_some() => Ok(Some(s.clone())),
            Some(_) => Err(bad(format!("{key} must be YYYY-MM-DD"))),
        }
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

/// Collects `column = ?` assignments for an UPDATE.
#[derive(Default)]
struct Patch {
    sets: Vec<String>,
    values: Vec<Sql>,
}

impl Patch {
    fn set(&mut self, column: &str, value: Sql) {
        self.sets.push(format!("{column} = ?"));
        self.values.push(value);
    }

    fn text(&mut self, column: &str, value: String) {
        self.set(column, Sql::Text(value));
    }

    fn opt_text(&mut self, column: &str, value: Option<String>) {
        self.set(column, value.map_or(Sql::Null, Sql::Text));
    }

    fn int(&mut self, column: &str, value: i64) {
        self.set(column, Sql::Integer(value));
    }

    fn opt_int(&mut self, column: &str, value: Option<i64>) {
        self.set(column, value.map_or(Sql::Null, Sql::Integer));
    }

    /// Applies the assignments to one row; `touch` also bumps updated_at.
    fn apply(mut self, conn: &Connection, table: &str, id: i64, touch: bool) -> rusqlite::Result<()> {
        if self.sets.is_empty() {
            return Ok(());
        }
        if touch {
            self.sets.push("updated_at = datetime('now')".into());
        }
        self.values.push(Sql::Integer(id));
        conn.execute(&format!("UPDATE {table} SET {} WHERE id = ?", self.sets.join(", ")), params_from_iter(self.values))?;
        Ok(())
    }
}

/// The next position at the end of a list.
fn next_position(conn: &Connection, sql: &str, p: impl rusqlite::Params) -> rusqlite::Result<i64> {
    conn.query_row(sql, p, |r| r.get::<_, Option<i64>>(0)).map(|n| n.map_or(0, |n| n + 1))
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

fn flag(v: Option<&Value>) -> i64 {
    i64::from(v.is_some_and(truthy))
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

/// Like clean_text but keeps surrounding whitespace (descriptions, notes).
fn clean_long(v: Option<&Value>, field: &str, max_len: usize) -> ApiResult<String> {
    match v {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) if s.chars().count() > max_len => Err(bad(format!("{field} is too long"))),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(bad(format!("{field} must be text"))),
    }
}

fn clean_date(v: Option<&Value>, field: &str) -> ApiResult<Option<String>> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(Value::String(s)) if recur::parse_date(s).is_some() => Ok(Some(s.clone())),
        Some(_) => Err(bad(format!("{field} must be YYYY-MM-DD"))),
    }
}

fn valid_time(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && s[0..2].parse::<u8>().is_ok_and(|h| h < 24 && b[0].is_ascii_digit() && b[1].is_ascii_digit())
        && s[3..5].parse::<u8>().is_ok_and(|m| m < 60 && b[3].is_ascii_digit() && b[4].is_ascii_digit())
}

fn clean_time(v: Option<&Value>, field: &str) -> ApiResult<Option<String>> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(Value::String(s)) if valid_time(s) => Ok(Some(s.clone())),
        Some(_) => Err(bad(format!("{field} must be HH:MM"))),
    }
}

/// "YYYY-MM-DDTHH:MM", local time with no zone.
fn valid_datetime(s: &str) -> bool {
    s.len() == 16 && s.as_bytes()[10] == b'T' && recur::parse_date(&s[..10]).is_some() && valid_time(&s[11..])
}

fn clean_datetime(v: Option<&Value>, field: &str) -> ApiResult<Option<String>> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(Value::String(s)) if valid_datetime(s) => Ok(Some(s.clone())),
        Some(_) => Err(bad(format!("{field} must be YYYY-MM-DDTHH:MM"))),
    }
}

fn clean_priority(v: Option<&Value>) -> ApiResult<i64> {
    match v.and_then(Value::as_i64) {
        Some(n) if (0..=3).contains(&n) => Ok(n),
        _ => Err(bad("priority must be 0 (none) to 3 (high)")),
    }
}

fn clean_color(v: Option<&Value>, default: &str) -> ApiResult<String> {
    match v {
        None | Some(Value::Null) => Ok(default.into()),
        Some(Value::String(s)) if s.is_empty() => Ok(default.into()),
        Some(Value::String(s)) if COLORS.contains(&s.as_str()) => Ok(s.clone()),
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

fn int_field(v: &Value, field: &str) -> ApiResult<i64> {
    v.as_i64().ok_or_else(|| bad(format!("{field} must be an integer")))
}

fn owns(conn: &Connection, table: &str, uid: i64, id: i64) -> rusqlite::Result<bool> {
    exists(conn, &format!("SELECT 1 FROM {table} WHERE id = ?1 AND user_id = ?2"), params![id, uid])
}

fn owned_class(conn: &Connection, uid: i64, v: Option<&Value>) -> ApiResult<Option<i64>> {
    let Some(id) = opt_id(v, "class_id")? else { return Ok(None) };
    if owns(conn, "classes", uid, id)? {
        Ok(Some(id))
    } else {
        Err(not_found("Class"))
    }
}

fn require(conn: &Connection, table: &str, what: &str, uid: i64, id: i64) -> ApiResult<Value> {
    one(conn, &format!("SELECT * FROM {table} WHERE id = ?1 AND user_id = ?2"), params![id, uid])?.ok_or_else(|| not_found(what))
}

/// Today's date as the client sees it when it says, otherwise the server's (UTC) date.
fn today(v: Option<&Value>) -> i64 {
    v.and_then(Value::as_str).and_then(recur::parse_date).unwrap_or_else(|| auth::unix_now().div_euclid(86_400))
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

/// Everything the app shows on first paint, in one round trip.
fn bootstrap(ctx: &Ctx) -> ApiResult<Reply> {
    let user = ctx.user.as_ref().ok_or_else(|| ApiError::new(401, "Sign in first"))?;
    let today = recur::format_date(today(ctx.query.get("today").map(|s| Value::String(s.clone())).as_ref()));
    let db = ctx.db();
    ok(json!({
        "user": { "id": user.id, "username": user.username },
        "classes": classes::list(&db, user.id)?,
        "sections": tasks::sections(&db, user.id)?,
        "todos": tasks::list(&db, user.id, "recent")?,
        "pages": pages::tree(&db, user.id, false)?,
        "habits": calendar::habits(&db, user.id)?,
        "countdowns": calendar::countdowns(&db, user.id, &today)?,
    }))
}

// ---------- routing ----------

type Handler = fn(&Ctx) -> ApiResult<Reply>;

struct Route {
    method: &'static str,
    pattern: &'static str,
    handler: Handler,
    needs_auth: bool,
    /// The body is raw bytes, not JSON.
    raw: bool,
}

const fn route(method: &'static str, pattern: &'static str, handler: Handler, needs_auth: bool) -> Route {
    Route { method, pattern, handler, needs_auth, raw: false }
}

static ROUTES: &[Route] = &[
    route("POST", "/api/signup", signup, false),
    route("POST", "/api/login", login, false),
    route("POST", "/api/logout", logout, false),
    route("GET", "/api/me", me, true),
    route("GET", "/api/bootstrap", bootstrap, true),
    route("GET", "/api/search", search::search, true),
    route("POST", "/api/reorder", tasks::reorder, true),
    route("GET", "/api/todos", tasks::todos_list, true),
    route("POST", "/api/todos", tasks::todo_create, true),
    route("GET", "/api/todos/:id", tasks::todo_get, true),
    route("PATCH", "/api/todos/:id", tasks::todo_update, true),
    route("DELETE", "/api/todos/:id", tasks::todo_delete, true),
    route("POST", "/api/todos/:id/complete", tasks::todo_complete, true),
    route("GET", "/api/sections", tasks::sections_list, true),
    route("POST", "/api/sections", tasks::section_create, true),
    route("PATCH", "/api/sections/:id", tasks::section_update, true),
    route("DELETE", "/api/sections/:id", tasks::section_delete, true),
    route("GET", "/api/classes", classes::classes_list, true),
    route("POST", "/api/classes", classes::class_create, true),
    route("GET", "/api/classes/:id", classes::class_get, true),
    route("PATCH", "/api/classes/:id", classes::class_update, true),
    route("DELETE", "/api/classes/:id", classes::class_delete, true),
    route("GET", "/api/pages", pages::pages_list, true),
    route("POST", "/api/pages", pages::page_create, true),
    route("GET", "/api/pages/:id", pages::page_get, true),
    route("PATCH", "/api/pages/:id", pages::page_update, true),
    route("DELETE", "/api/pages/:id", pages::page_delete, true),
    route("GET", "/api/events", calendar::events_list, true),
    route("POST", "/api/events", calendar::event_create, true),
    route("PATCH", "/api/events/:id", calendar::event_update, true),
    route("DELETE", "/api/events/:id", calendar::event_delete, true),
    route("GET", "/api/countdowns", calendar::countdowns_list, true),
    route("GET", "/api/habits", calendar::habits_list, true),
    route("POST", "/api/habits", calendar::habit_create, true),
    route("PATCH", "/api/habits/:id", calendar::habit_update, true),
    route("DELETE", "/api/habits/:id", calendar::habit_delete, true),
    route("POST", "/api/habits/:id/check", calendar::habit_check, true),
    Route { method: "POST", pattern: "/api/files", handler: files::upload, needs_auth: true, raw: true },
    route("GET", "/api/files/:id", files::download, true),
];

/// Largest accepted request body for a route: uploads get more room than JSON.
pub fn body_limit(method: &str, path: &str) -> usize {
    if method == "POST" && path == "/api/files" {
        files::MAX_UPLOAD_BYTES
    } else {
        4_000_000
    }
}

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
    percent_decode(&text.replace('+', " "))
}

fn percent_decode(text: &str) -> String {
    let raw = text.as_bytes();
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
    if !route.raw && matches!(req.method, "POST" | "PATCH" | "PUT" | "DELETE") && !raw_body.is_empty() {
        let is_json = req.headers.get("content-type").is_some_and(|c| c.contains("application/json"));
        match serde_json::from_slice::<Value>(raw_body) {
            Ok(Value::Object(map)) if is_json => body = Some(map),
            _ => return error_reply(bad("Body must be a JSON object")),
        }
    }

    let ctx = Ctx { state, user, body, raw: raw_body, headers: req.headers, query: parse_query(req.query), params, token };
    match (route.handler)(&ctx) {
        Ok(reply) => reply,
        Err(err) => error_reply(err),
    }
}
