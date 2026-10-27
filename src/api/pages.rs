//! Pages: nested documents stored as editor JSON, plus a plain-text copy for search and the
//! mentions they contain (for backlinks).

use rusqlite::{params, Connection};
use serde_json::{json, Value};

use super::*;

const MAX_CONTENT_BYTES: usize = 3_500_000;
const MAX_TEXT_CHARS: usize = 400_000;

/// Columns for listings: everything except the document itself.
const META: &str = "id, parent_id, todo_id, class_id, title, icon, favorite, position, archived, updated_at";

pub(super) fn tree(conn: &Connection, uid: i64, archived: bool) -> rusqlite::Result<Vec<Value>> {
    rows(
        conn,
        &format!("SELECT {META} FROM pages WHERE user_id = ?1 AND archived = ?2 ORDER BY position, id"),
        params![uid, i64::from(archived)],
    )
}

fn require_page(conn: &Connection, uid: i64, id: i64) -> ApiResult<Value> {
    require(conn, "pages", "Page", uid, id)
}

/// Validates a new parent for page `id` (None while creating): owned, and not inside the page itself.
pub(super) fn checked_parent(conn: &Connection, uid: i64, id: i64, v: Option<&Value>) -> ApiResult<Option<i64>> {
    let Some(parent) = opt_id(v, "parent_id")? else { return Ok(None) };
    if !owns(conn, "pages", uid, parent)? {
        return Err(not_found("Parent page"));
    }
    let mut cursor = Some(parent);
    for _ in 0..1000 {
        match cursor {
            Some(p) if p == id => return Err(bad("A page cannot be moved inside itself")),
            Some(p) => cursor = conn.query_row("SELECT parent_id FROM pages WHERE id = ?1", params![p], |r| r.get(0))?,
            None => break,
        }
    }
    Ok(Some(parent))
}

fn clean_links(v: &Value) -> ApiResult<Vec<(String, i64)>> {
    let list = v.as_array().ok_or_else(|| bad("links must be a list"))?;
    let mut out = Vec::new();
    for link in list.iter().take(2000) {
        let kind = link.get("kind").and_then(Value::as_str).filter(|k| matches!(*k, "page" | "todo" | "class"));
        let id = link.get("id").and_then(Value::as_i64);
        match (kind, id) {
            (Some(k), Some(id)) => out.push((k.to_string(), id)),
            _ => return Err(bad("each link needs a kind (page, todo or class) and an id")),
        }
    }
    Ok(out)
}

pub(super) fn pages_list(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let db = ctx.db();
    let list = if let Some(id) = ctx.query_id("todo_id")? {
        require(&db, "todos", "To-do", uid, id)?;
        rows(&db, &format!("SELECT {META} FROM pages WHERE user_id = ?1 AND todo_id = ?2 AND archived = 0 ORDER BY position, id"), params![uid, id])?
    } else if let Some(id) = ctx.query_id("class_id")? {
        require(&db, "classes", "Class", uid, id)?;
        rows(&db, &format!("SELECT {META} FROM pages WHERE user_id = ?1 AND class_id = ?2 AND archived = 0 ORDER BY position, id"), params![uid, id])?
    } else {
        tree(&db, uid, ctx.query.get("archived").is_some_and(|v| v == "1"))?
    };
    ok(Value::Array(list))
}

pub(super) fn page_create(ctx: &Ctx) -> ApiResult<Reply> {
    let body = ctx.body()?;
    let uid = ctx.uid()?;
    let title = clean_text(body.get("title"), "title", 200, false)?;
    let icon = clean_text(body.get("icon"), "icon", 16, false)?;
    let db = ctx.db();
    let tx = db.unchecked_transaction()?;
    let parent = checked_parent(&tx, uid, -1, body.get("parent_id"))?;
    // A sub-page belongs to its parent; only top-level pages hang off a to-do or a class.
    let (todo_id, class_id) = if parent.is_some() {
        (None, None)
    } else {
        let todo_id = opt_id(body.get("todo_id"), "todo_id")?;
        if let Some(t) = todo_id {
            require(&tx, "todos", "To-do", uid, t)?;
        }
        let class_id = if todo_id.is_some() { None } else { owned_class(&tx, uid, body.get("class_id"))? };
        (todo_id, class_id)
    };
    let position = next_position(
        &tx,
        "SELECT MAX(position) FROM pages WHERE user_id = ?1 AND parent_id IS ?2 AND todo_id IS ?3 AND class_id IS ?4",
        params![uid, parent, todo_id, class_id],
    )?;
    tx.execute(
        "INSERT INTO pages (user_id, parent_id, todo_id, class_id, title, icon, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![uid, parent, todo_id, class_id, title, icon, position],
    )?;
    let id = tx.last_insert_rowid();
    let mut patch = Patch::default();
    content_fields(&tx, uid, id, body, &mut patch)?;
    patch.apply(&tx, "pages", id, false)?;
    tx.commit()?;
    created(require_page(&db, uid, id)?)
}

/// The document fields shared by create and update; also rewrites the page's links.
fn content_fields(conn: &Connection, uid: i64, id: i64, body: &serde_json::Map<String, Value>, patch: &mut Patch) -> ApiResult<()> {
    if let Some(v) = body.get("content") {
        let json = if v.is_null() { None } else { Some(v.to_string()) };
        if json.as_ref().is_some_and(|j| j.len() > MAX_CONTENT_BYTES) {
            return Err(bad("document is too long"));
        }
        patch.opt_text("content", json);
    }
    if body.contains_key("text") {
        patch.text("text", clean_long(body.get("text"), "text", MAX_TEXT_CHARS)?);
    }
    if body.contains_key("body") {
        patch.text("body", clean_long(body.get("body"), "body", MAX_TEXT_CHARS)?);
    }
    if let Some(v) = body.get("links") {
        let links = clean_links(v)?;
        conn.execute("DELETE FROM links WHERE from_page = ?1", params![id])?;
        for (kind, to) in links {
            conn.execute(
                "INSERT OR IGNORE INTO links (user_id, from_page, to_kind, to_id) VALUES (?1, ?2, ?3, ?4)",
                params![uid, id, kind, to],
            )?;
        }
    }
    Ok(())
}

pub(super) fn page_get(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    let mut page = require_page(&db, uid, id)?;
    if let Some(content) = page["content"].as_str().and_then(|s| serde_json::from_str::<Value>(s).ok()) {
        page["content"] = content;
    }
    page["children"] = Value::Array(rows(
        &db,
        &format!("SELECT {META} FROM pages WHERE parent_id = ?1 AND archived = 0 ORDER BY position, id"),
        params![id],
    )?);
    page["backlinks"] = Value::Array(rows(
        &db,
        "SELECT DISTINCT p.id, p.title, p.icon FROM links l JOIN pages p ON p.id = l.from_page
         WHERE l.user_id = ?1 AND l.to_kind = 'page' AND l.to_id = ?2 AND p.archived = 0",
        params![uid, id],
    )?);
    // Ancestors, root first, for the breadcrumb.
    page["path"] = Value::Array(rows(
        &db,
        "WITH RECURSIVE up(id, parent_id, title, icon, todo_id, class_id, depth) AS (
             SELECT id, parent_id, title, icon, todo_id, class_id, 0 FROM pages WHERE id = ?1
             UNION ALL SELECT p.id, p.parent_id, p.title, p.icon, p.todo_id, p.class_id, up.depth + 1
             FROM pages p JOIN up ON p.id = up.parent_id WHERE up.depth < 50)
         SELECT id, title, icon, todo_id, class_id FROM up WHERE id != ?1 ORDER BY depth DESC",
        params![id],
    )?);
    ok(page)
}

pub(super) fn page_update(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    require_page(&db, uid, id)?;
    let body = ctx.body()?;
    let tx = db.unchecked_transaction()?;
    let mut patch = Patch::default();
    if let Some(v) = body.get("title") {
        patch.text("title", clean_text(Some(v), "title", 200, false)?);
    }
    if body.contains_key("icon") {
        patch.text("icon", clean_text(body.get("icon"), "icon", 16, false)?);
    }
    if let Some(v) = body.get("font") {
        match v.as_str() {
            Some(f @ ("default" | "serif" | "mono")) => patch.text("font", f.into()),
            _ => return Err(bad("font must be default, serif or mono")),
        }
    }
    for key in ["full_width", "favorite", "archived"] {
        if body.contains_key(key) {
            patch.int(key, flag(body.get(key)));
        }
    }
    if let Some(v) = body.get("position") {
        patch.int("position", int_field(v, "position")?);
    }
    if body.contains_key("parent_id") {
        let parent = checked_parent(&tx, uid, id, body.get("parent_id"))?;
        patch.opt_int("parent_id", parent);
        if parent.is_some() {
            patch.opt_int("todo_id", None);
            patch.opt_int("class_id", None);
        }
    }
    content_fields(&tx, uid, id, body, &mut patch)?;
    let archive = body.get("archived").map(truthy);
    patch.apply(&tx, "pages", id, true)?;
    // Trashing or restoring a page does the same to everything inside it.
    if let Some(archived) = archive {
        tx.execute(
            "WITH RECURSIVE sub(id) AS (SELECT id FROM pages WHERE parent_id = ?1
                 UNION SELECT p.id FROM pages p JOIN sub ON p.parent_id = sub.id)
             UPDATE pages SET archived = ?2 WHERE id IN sub",
            params![id, i64::from(archived)],
        )?;
    }
    tx.commit()?;
    let mut page = require_page(&db, uid, id)?;
    page.as_object_mut().map(|o| o.remove("content"));
    ok(json!(page))
}

pub(super) fn page_delete(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let db = ctx.db();
    require_page(&db, uid, ctx.params[0])?;
    db.execute("DELETE FROM pages WHERE id = ?1", params![ctx.params[0]])?;
    done()
}
