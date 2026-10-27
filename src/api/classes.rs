//! Classes: Todoist-style projects with a colour, sections, a list or board view and a home page.

use rusqlite::{params, Connection};
use serde_json::{json, Value};

use super::*;

pub(super) fn list(conn: &Connection, uid: i64) -> rusqlite::Result<Vec<Value>> {
    rows(
        conn,
        "SELECT c.*,
                COUNT(t.id) AS task_count,
                COALESCE(SUM(CASE WHEN t.done = 0 THEN 1 ELSE 0 END), 0) AS open_count,
                (SELECT p.id FROM pages p WHERE p.class_id = c.id AND p.parent_id IS NULL AND p.archived = 0
                  ORDER BY p.position, p.id LIMIT 1) AS home_page_id
         FROM classes c LEFT JOIN todos t ON t.class_id = c.id
         WHERE c.user_id = ?1
         GROUP BY c.id
         ORDER BY c.archived, c.position, lower(c.name)",
        params![uid],
    )
}

fn require_class(conn: &Connection, uid: i64, id: i64) -> ApiResult<Value> {
    require(conn, "classes", "Class", uid, id)
}

pub(super) fn classes_list(ctx: &Ctx) -> ApiResult<Reply> {
    ok(Value::Array(list(&ctx.db(), ctx.uid()?)?))
}

pub(super) fn class_create(ctx: &Ctx) -> ApiResult<Reply> {
    let body = ctx.body()?;
    let uid = ctx.uid()?;
    let name = clean_text(body.get("name"), "name", 120, true)?;
    let section = clean_text(body.get("section"), "section", 120, false)?;
    let color = clean_color(body.get("color"), "charcoal")?;
    let emoji = clean_text(body.get("emoji"), "emoji", 16, false)?;
    let db = ctx.db();
    let tx = db.unchecked_transaction()?;
    let position = next_position(&tx, "SELECT MAX(position) FROM classes WHERE user_id = ?1", params![uid])?;
    tx.execute(
        "INSERT INTO classes (user_id, name, section, color, emoji, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![uid, name, section, color, emoji, position],
    )?;
    let id = tx.last_insert_rowid();
    // Every class has a home page: its central document.
    tx.execute("INSERT INTO pages (user_id, class_id, title) VALUES (?1, ?2, ?3)", params![uid, id, name])?;
    tx.commit()?;
    created(require_class(&db, uid, id)?)
}

pub(super) fn class_get(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    let class = require_class(&db, uid, id)?;
    let todos = rows(
        &db,
        "SELECT t.*, c.name AS class_name, c.color AS class_color FROM todos t
         LEFT JOIN classes c ON c.id = t.class_id
         WHERE t.user_id = ?1 AND t.class_id = ?2 ORDER BY t.done, t.position, t.id",
        params![uid, id],
    )?;
    let sections = rows(&db, "SELECT * FROM sections WHERE class_id = ?1 ORDER BY position, id", params![id])?;
    let pages = rows(
        &db,
        "SELECT id, title, icon, position, updated_at FROM pages
         WHERE class_id = ?1 AND parent_id IS NULL AND archived = 0 ORDER BY position, id",
        params![id],
    )?;
    ok(json!({ "class": class, "todos": todos, "sections": sections, "pages": pages }))
}

pub(super) fn class_update(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    require_class(&db, uid, id)?;
    let body = ctx.body()?;
    let mut patch = Patch::default();
    if let Some(v) = body.get("name") {
        patch.text("name", clean_text(Some(v), "name", 120, true)?);
    }
    if let Some(v) = body.get("section") {
        patch.text("section", clean_text(Some(v), "section", 120, false)?);
    }
    if body.contains_key("color") {
        patch.text("color", clean_color(body.get("color"), "charcoal")?);
    }
    if body.contains_key("emoji") {
        patch.text("emoji", clean_text(body.get("emoji"), "emoji", 16, false)?);
    }
    if let Some(v) = body.get("view") {
        match v.as_str() {
            Some(view @ ("list" | "board")) => patch.text("view", view.into()),
            _ => return Err(bad("view must be list or board")),
        }
    }
    if body.contains_key("archived") {
        patch.int("archived", flag(body.get("archived")));
    }
    if let Some(v) = body.get("position") {
        patch.int("position", int_field(v, "position")?);
    }
    patch.apply(&db, "classes", id, true)?;
    ok(require_class(&db, uid, id)?)
}

pub(super) fn class_delete(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let db = ctx.db();
    require_class(&db, uid, ctx.params[0])?;
    db.execute("DELETE FROM classes WHERE id = ?1", params![ctx.params[0]])?;
    done()
}
