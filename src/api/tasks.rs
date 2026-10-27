//! To-dos (with subtasks, sections, scheduling and recurrence), sections, and drag-and-drop reordering.

use rusqlite::types::Value as Sql;
use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};

use super::*;

const TODO_SELECT: &str = "SELECT t.*, c.name AS class_name, c.color AS class_color
    FROM todos t LEFT JOIN classes c ON c.id = t.class_id";

/// Ids of a task's subtasks at any depth.
const DESCENDANTS: &str = "WITH RECURSIVE sub(id) AS (
        SELECT id FROM todos WHERE parent_id = ?1
        UNION SELECT t.id FROM todos t JOIN sub ON t.parent_id = sub.id)
    SELECT id FROM sub";

pub(super) fn todo_row(conn: &Connection, id: i64) -> ApiResult<Value> {
    one(conn, &format!("{TODO_SELECT} WHERE t.id = ?1"), params![id])?.ok_or_else(|| not_found("To-do"))
}

/// "open", "done", "all", or "recent" (open plus anything finished in the last two weeks).
pub(super) fn list(conn: &Connection, uid: i64, which: &str) -> rusqlite::Result<Vec<Value>> {
    let filter = match which {
        "open" => "AND t.done = 0",
        "done" => "AND t.done = 1",
        "all" => "",
        _ => "AND (t.done = 0 OR t.completed_at >= datetime('now', '-14 days'))",
    };
    rows(conn, &format!("{TODO_SELECT} WHERE t.user_id = ?1 {filter} ORDER BY t.done, t.position, t.id"), params![uid])
}

fn require_todo(conn: &Connection, uid: i64, id: i64) -> ApiResult<Value> {
    require(conn, "todos", "To-do", uid, id)
}

fn descendants(conn: &Connection, id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut stmt = conn.prepare(DESCENDANTS)?;
    let ids = stmt.query_map(params![id], |r| r.get(0))?;
    ids.collect()
}

fn opt_i64(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(Value::as_i64)
}

/// Validates the task fields present in `body` into `patch`. `id` is None while creating.
/// Returns true when the task's place (class, section or parent) changed.
fn task_fields(conn: &Connection, uid: i64, id: Option<i64>, body: &Map<String, Value>, patch: &mut Patch) -> ApiResult<bool> {
    if let Some(v) = body.get("title") {
        patch.text("title", clean_text(Some(v), "title", 500, true)?);
    }
    if body.contains_key("description") {
        patch.text("description", clean_long(body.get("description"), "description", 20_000)?);
    }
    if body.contains_key("due_date") {
        patch.opt_text("due_date", clean_date(body.get("due_date"), "due_date")?);
    }
    if body.contains_key("due_time") {
        patch.opt_text("due_time", clean_time(body.get("due_time"), "due_time")?);
    }
    if body.contains_key("priority") {
        patch.int("priority", clean_priority(body.get("priority"))?);
    }
    if body.contains_key("position") {
        patch.int("position", int_field(&body["position"], "position")?);
    }
    if body.contains_key("recurrence") {
        let rule = match body.get("recurrence") {
            None | Some(Value::Null) => None,
            Some(v) => Some(recur::Rule::from_json(v).map_err(bad)?.to_json().to_string()),
        };
        patch.opt_text("recurrence", rule);
    }
    let start = if body.contains_key("start_at") { Some(clean_datetime(body.get("start_at"), "start_at")?) } else { None };
    let end = if body.contains_key("end_at") { Some(clean_datetime(body.get("end_at"), "end_at")?) } else { None };
    if let (Some(Some(s)), Some(Some(e))) = (&start, &end) {
        if e <= s {
            return Err(bad("end_at must be after start_at"));
        }
    }
    if let Some(s) = start {
        patch.opt_text("start_at", s);
    }
    if let Some(e) = end {
        patch.opt_text("end_at", e);
    }

    // Place: a parent wins (subtasks live with their parent), then a section, then a class.
    let mut moved = false;
    if body.contains_key("parent_id") {
        let parent = opt_id(body.get("parent_id"), "parent_id")?;
        if let Some(pid) = parent {
            let p = require_todo(conn, uid, pid).map_err(|_| not_found("Parent task"))?;
            if let Some(id) = id {
                if pid == id || descendants(conn, id)?.contains(&pid) {
                    return Err(bad("A task cannot be its own subtask"));
                }
            }
            patch.opt_int("class_id", opt_i64(&p, "class_id"));
            patch.opt_int("section_id", opt_i64(&p, "section_id"));
        }
        patch.opt_int("parent_id", parent);
        moved = true;
    }
    if !body.get("parent_id").is_some_and(|v| !v.is_null()) {
        if let Some(sid) = opt_id(body.get("section_id"), "section_id")? {
            let s = require(conn, "sections", "Section", uid, sid)?;
            patch.opt_int("class_id", opt_i64(&s, "class_id"));
            patch.int("section_id", sid);
            moved = true;
        } else if body.contains_key("class_id") {
            patch.opt_int("class_id", owned_class(conn, uid, body.get("class_id"))?);
            patch.opt_int("section_id", None);
            moved = true;
        } else if body.contains_key("section_id") {
            patch.opt_int("section_id", None);
            moved = true;
        }
    }
    Ok(moved)
}

/// Subtasks follow their parent's class and section.
fn carry_children(conn: &Connection, id: i64) -> ApiResult<()> {
    let row = todo_row(conn, id)?;
    for child in descendants(conn, id)? {
        conn.execute(
            "UPDATE todos SET class_id = ?1, section_id = ?2 WHERE id = ?3",
            params![opt_i64(&row, "class_id"), opt_i64(&row, "section_id"), child],
        )?;
    }
    Ok(())
}

pub(super) fn todos_list(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let which = ctx.query.get("done").map(String::as_str).unwrap_or("recent");
    let which = match which {
        "0" => "open",
        "1" => "done",
        other => other,
    };
    ok(Value::Array(list(&ctx.db(), uid, which)?))
}

pub(super) fn todo_create(ctx: &Ctx) -> ApiResult<Reply> {
    let body = ctx.body()?;
    let uid = ctx.uid()?;
    let title = clean_text(body.get("title"), "title", 500, true)?;
    let db = ctx.db();
    let tx = db.unchecked_transaction()?;
    let mut patch = Patch::default();
    task_fields(&tx, uid, None, body, &mut patch)?;
    let position = next_position(&tx, "SELECT MAX(position) FROM todos WHERE user_id = ?1", params![uid])?;
    tx.execute("INSERT INTO todos (user_id, title, position) VALUES (?1, ?2, ?3)", params![uid, title, position])?;
    let id = tx.last_insert_rowid();
    patch.apply(&tx, "todos", id, false)?;
    tx.commit()?;
    created(todo_row(&db, id)?)
}

pub(super) fn todo_get(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    require_todo(&db, uid, id)?;
    let mut row = todo_row(&db, id)?;
    row["subtasks"] = Value::Array(rows(&db, &format!("{TODO_SELECT} WHERE t.parent_id = ?1 ORDER BY t.position, t.id"), params![id])?);
    row["pages"] = Value::Array(rows(
        &db,
        "SELECT id, title, icon, position FROM pages WHERE todo_id = ?1 AND archived = 0 ORDER BY position, id",
        params![id],
    )?);
    ok(row)
}

fn set_done(conn: &Connection, ids: &[i64], done: bool) -> rusqlite::Result<()> {
    let when = if done { Sql::Text(now_sql(conn)?) } else { Sql::Null };
    for id in ids {
        conn.execute(
            "UPDATE todos SET done = ?1, completed_at = ?2, updated_at = datetime('now') WHERE id = ?3",
            rusqlite::params![i64::from(done), when, id],
        )?;
    }
    Ok(())
}

pub(super) fn todo_update(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    require_todo(&db, uid, id)?;
    let body = ctx.body()?;
    let tx = db.unchecked_transaction()?;
    let mut patch = Patch::default();
    let moved = task_fields(&tx, uid, Some(id), body, &mut patch)?;
    patch.apply(&tx, "todos", id, true)?;
    if let Some(v) = body.get("done") {
        // Finishing a task finishes its subtasks too; reopening only reopens the task itself.
        let finished = truthy(v);
        let mut ids = vec![id];
        if finished {
            ids.extend(descendants(&tx, id)?);
        }
        set_done(&tx, &ids, finished)?;
    }
    if moved {
        carry_children(&tx, id)?;
    }
    tx.commit()?;
    ok(todo_row(&db, id)?)
}

/// Checking off a task. A recurring one moves to its next date (and its subtasks reopen) instead.
pub(super) fn todo_complete(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    let row = require_todo(&db, uid, id)?;
    let today = today(ctx.body.as_ref().and_then(|b| b.get("today")));
    let rule = row["recurrence"].as_str().and_then(|s| serde_json::from_str::<Value>(s).ok()).and_then(|v| recur::Rule::from_json(&v).ok());

    let tx = db.unchecked_transaction()?;
    let recurred = if let Some(rule) = rule {
        let due = row["due_date"].as_str().and_then(recur::parse_date).unwrap_or(today);
        let next = recur::format_date(rule.advance(due, today));
        tx.execute(
            "UPDATE todos SET due_date = ?1, start_at = NULL, end_at = NULL, done = 0, completed_at = NULL,
                updated_at = datetime('now') WHERE id = ?2",
            params![next, id],
        )?;
        set_done(&tx, &descendants(&tx, id)?, false)?;
        true
    } else {
        let mut ids = vec![id];
        ids.extend(descendants(&tx, id)?);
        set_done(&tx, &ids, true)?;
        false
    };
    tx.commit()?;
    ok(json!({ "todo": todo_row(&db, id)?, "recurred": recurred }))
}

pub(super) fn todo_delete(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let db = ctx.db();
    require_todo(&db, uid, ctx.params[0])?;
    db.execute("DELETE FROM todos WHERE id = ?1", params![ctx.params[0]])?;
    done()
}

// ---------- sections ----------

pub(super) fn sections(conn: &Connection, uid: i64) -> rusqlite::Result<Vec<Value>> {
    rows(conn, "SELECT * FROM sections WHERE user_id = ?1 ORDER BY position, id", params![uid])
}

pub(super) fn sections_list(ctx: &Ctx) -> ApiResult<Reply> {
    ok(Value::Array(sections(&ctx.db(), ctx.uid()?)?))
}

pub(super) fn section_create(ctx: &Ctx) -> ApiResult<Reply> {
    let body = ctx.body()?;
    let uid = ctx.uid()?;
    let name = clean_text(body.get("name"), "name", 120, true)?;
    let db = ctx.db();
    let class_id = owned_class(&db, uid, body.get("class_id"))?;
    let position = match body.get("position") {
        Some(v) => int_field(v, "position")?,
        None => next_position(&db, "SELECT MAX(position) FROM sections WHERE user_id = ?1 AND class_id IS ?2", params![uid, class_id])?,
    };
    db.execute(
        "INSERT INTO sections (user_id, class_id, name, position) VALUES (?1, ?2, ?3, ?4)",
        params![uid, class_id, name, position],
    )?;
    created(require(&db, "sections", "Section", uid, db.last_insert_rowid())?)
}

pub(super) fn section_update(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    require(&db, "sections", "Section", uid, id)?;
    let body = ctx.body()?;
    let mut patch = Patch::default();
    if let Some(v) = body.get("name") {
        patch.text("name", clean_text(Some(v), "name", 120, true)?);
    }
    if let Some(v) = body.get("position") {
        patch.int("position", int_field(v, "position")?);
    }
    if body.contains_key("collapsed") {
        patch.int("collapsed", flag(body.get("collapsed")));
    }
    patch.apply(&db, "sections", id, false)?;
    ok(require(&db, "sections", "Section", uid, id)?)
}

pub(super) fn section_delete(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    require(&db, "sections", "Section", uid, id)?;
    // Todoist deletes a section's tasks with it.
    let tx = db.unchecked_transaction()?;
    tx.execute("DELETE FROM todos WHERE section_id = ?1 AND user_id = ?2", params![id, uid])?;
    tx.execute("DELETE FROM sections WHERE id = ?1", params![id])?;
    tx.commit()?;
    done()
}

// ---------- reordering ----------

/// Saves a drag and drop: {"kind": "todos" | "sections" | "classes" | "pages" | "habits",
/// "items": [{"id": 1, "position": 0, ...}]}. Tasks may also change section, class or parent;
/// pages may change parent.
pub(super) fn reorder(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let body = ctx.body()?;
    let table = match body.get("kind").and_then(Value::as_str) {
        Some(k @ ("todos" | "sections" | "classes" | "pages" | "habits")) => k,
        _ => return Err(bad("kind must be todos, sections, classes, pages or habits")),
    };
    let items = body.get("items").and_then(Value::as_array).ok_or_else(|| bad("items must be a list"))?;
    if items.len() > 2000 {
        return Err(bad("Too many items"));
    }
    let db = ctx.db();
    let tx = db.unchecked_transaction()?;
    for item in items {
        let item = item.as_object().ok_or_else(|| bad("Each item must be an object"))?;
        let id = int_field(item.get("id").unwrap_or(&Value::Null), "id")?;
        if !owns(&tx, table, uid, id)? {
            return Err(not_found("Item"));
        }
        let mut patch = Patch::default();
        if let Some(v) = item.get("position") {
            patch.int("position", int_field(v, "position")?);
        }
        match table {
            "todos" => {
                let mut place = Map::new();
                for key in ["section_id", "class_id", "parent_id", "due_date"] {
                    if let Some(v) = item.get(key) {
                        place.insert(key.into(), v.clone());
                    }
                }
                let moved = task_fields(&tx, uid, Some(id), &place, &mut patch)?;
                patch.apply(&tx, table, id, true)?;
                if moved {
                    carry_children(&tx, id)?;
                }
            }
            "pages" => {
                if item.contains_key("parent_id") {
                    let parent = pages::checked_parent(&tx, uid, id, item.get("parent_id"))?;
                    patch.opt_int("parent_id", parent);
                }
                patch.apply(&tx, table, id, false)?;
            }
            _ => patch.apply(&tx, table, id, false)?,
        }
    }
    tx.commit()?;
    done()
}
