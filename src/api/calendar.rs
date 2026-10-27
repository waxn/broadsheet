//! Calendar events (and the countdowns among them) and habits.

use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};

use super::*;

// ---------- events ----------

fn require_event(conn: &Connection, uid: i64, id: i64) -> ApiResult<Value> {
    require(conn, "events", "Event", uid, id)
}

/// A start or end: a date for all-day events, a date and time otherwise.
fn clean_when(v: Option<&Value>, field: &str, all_day: bool) -> ApiResult<String> {
    let parsed = if all_day { clean_date(v, field)? } else { clean_datetime(v, field)? };
    parsed.ok_or_else(|| bad(format!("{field} is required")))
}

fn event_fields(conn: &Connection, uid: i64, body: &Map<String, Value>, current: Option<&Value>, patch: &mut Patch) -> ApiResult<()> {
    if let Some(v) = body.get("title") {
        patch.text("title", clean_text(Some(v), "title", 300, true)?);
    }
    let all_day = match body.get("all_day") {
        Some(v) => truthy(v),
        None => current.and_then(|c| c["all_day"].as_i64()).unwrap_or(0) == 1,
    };
    let changes_time = body.contains_key("start_at") || body.contains_key("end_at") || body.contains_key("all_day");
    if changes_time {
        let pick = |key: &str| body.get(key).or_else(|| current.map(|c| &c[key]));
        let start = clean_when(pick("start_at"), "start_at", all_day)?;
        let end = match pick("end_at") {
            Some(v) if !v.is_null() => clean_when(Some(v), "end_at", all_day)?,
            _ => start.clone(),
        };
        if if all_day { end < start } else { end <= start } {
            return Err(bad("end_at must be after start_at"));
        }
        patch.int("all_day", i64::from(all_day));
        patch.text("start_at", start);
        patch.text("end_at", end);
    }
    if body.contains_key("color") {
        let color = match body.get("color") {
            Some(Value::String(s)) if s.is_empty() => String::new(),
            other => clean_color(other, "")?,
        };
        patch.text("color", color);
    }
    if body.contains_key("class_id") {
        patch.opt_int("class_id", owned_class(conn, uid, body.get("class_id"))?);
    }
    if body.contains_key("location") {
        patch.text("location", clean_text(body.get("location"), "location", 300, false)?);
    }
    if body.contains_key("notes") {
        patch.text("notes", clean_long(body.get("notes"), "notes", 20_000)?);
    }
    if body.contains_key("countdown") {
        patch.int("countdown", flag(body.get("countdown")));
    }
    Ok(())
}

const EVENT_SELECT: &str = "SELECT e.*, c.name AS class_name, c.color AS class_color
    FROM events e LEFT JOIN classes c ON c.id = e.class_id";

fn event_row(conn: &Connection, id: i64) -> ApiResult<Value> {
    one(conn, &format!("{EVENT_SELECT} WHERE e.id = ?1"), params![id])?.ok_or_else(|| not_found("Event"))
}

pub(super) fn events_list(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let from = ctx.query_date("from")?.unwrap_or_else(|| "0000-01-01".into());
    let to = ctx.query_date("to")?.unwrap_or_else(|| "9999-12-31".into());
    let list = rows(
        &ctx.db(),
        &format!(
            "{EVENT_SELECT} WHERE e.user_id = ?1 AND substr(e.start_at, 1, 10) <= ?3 AND substr(e.end_at, 1, 10) >= ?2
             ORDER BY e.all_day DESC, e.start_at, e.id"
        ),
        params![uid, from, to],
    )?;
    ok(Value::Array(list))
}

pub(super) fn event_create(ctx: &Ctx) -> ApiResult<Reply> {
    let body = ctx.body()?;
    let uid = ctx.uid()?;
    let title = clean_text(body.get("title"), "title", 300, true)?;
    let db = ctx.db();
    let mut fields = body.clone();
    fields.entry("start_at").or_insert(Value::Null);
    let mut patch = Patch::default();
    event_fields(&db, uid, &fields, None, &mut patch)?;
    let tx = db.unchecked_transaction()?;
    tx.execute("INSERT INTO events (user_id, title, start_at, end_at) VALUES (?1, ?2, '', '')", params![uid, title])?;
    let id = tx.last_insert_rowid();
    patch.apply(&tx, "events", id, false)?;
    tx.commit()?;
    created(event_row(&db, id)?)
}

pub(super) fn event_update(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    let current = require_event(&db, uid, id)?;
    let mut patch = Patch::default();
    event_fields(&db, uid, ctx.body()?, Some(&current), &mut patch)?;
    patch.apply(&db, "events", id, true)?;
    ok(event_row(&db, id)?)
}

pub(super) fn event_delete(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let db = ctx.db();
    require_event(&db, uid, ctx.params[0])?;
    db.execute("DELETE FROM events WHERE id = ?1", params![ctx.params[0]])?;
    done()
}

pub(super) fn countdowns(conn: &Connection, uid: i64, today: &str) -> rusqlite::Result<Vec<Value>> {
    rows(
        conn,
        &format!("{EVENT_SELECT} WHERE e.user_id = ?1 AND e.countdown = 1 AND substr(e.start_at, 1, 10) >= ?2 ORDER BY e.start_at"),
        params![uid, today],
    )
}

pub(super) fn countdowns_list(ctx: &Ctx) -> ApiResult<Reply> {
    let today = recur::format_date(today(ctx.query.get("today").map(|s| Value::String(s.clone())).as_ref()));
    ok(Value::Array(countdowns(&ctx.db(), ctx.uid()?, &today)?))
}

// ---------- habits ----------

pub(super) fn habits(conn: &Connection, uid: i64) -> rusqlite::Result<Vec<Value>> {
    rows(conn, "SELECT * FROM habits WHERE user_id = ?1 ORDER BY archived, position, id", params![uid])
}

/// Habits and their check-ins between ?from and ?to (default: the last year).
pub(super) fn habits_list(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let today = today(None);
    let from = ctx.query_date("from")?.unwrap_or_else(|| recur::format_date(today - 400));
    let to = ctx.query_date("to")?.unwrap_or_else(|| recur::format_date(today + 30));
    let db = ctx.db();
    let checks = rows(
        &db,
        "SELECT hc.habit_id, hc.day FROM habit_checks hc JOIN habits h ON h.id = hc.habit_id
         WHERE h.user_id = ?1 AND hc.day BETWEEN ?2 AND ?3 ORDER BY hc.day",
        params![uid, from, to],
    )?;
    ok(json!({ "habits": habits(&db, uid)?, "checks": checks }))
}

fn habit_fields(body: &Map<String, Value>, patch: &mut Patch) -> ApiResult<()> {
    if let Some(v) = body.get("name") {
        patch.text("name", clean_text(Some(v), "name", 120, true)?);
    }
    if body.contains_key("emoji") {
        patch.text("emoji", clean_text(body.get("emoji"), "emoji", 16, false)?);
    }
    if body.contains_key("color") {
        patch.text("color", clean_color(body.get("color"), "green")?);
    }
    if body.contains_key("archived") {
        patch.int("archived", flag(body.get("archived")));
    }
    if let Some(v) = body.get("position") {
        patch.int("position", int_field(v, "position")?);
    }
    Ok(())
}

pub(super) fn habit_create(ctx: &Ctx) -> ApiResult<Reply> {
    let body = ctx.body()?;
    let uid = ctx.uid()?;
    let name = clean_text(body.get("name"), "name", 120, true)?;
    let mut patch = Patch::default();
    habit_fields(body, &mut patch)?;
    let db = ctx.db();
    let position = next_position(&db, "SELECT MAX(position) FROM habits WHERE user_id = ?1", params![uid])?;
    db.execute("INSERT INTO habits (user_id, name, position) VALUES (?1, ?2, ?3)", params![uid, name, position])?;
    let id = db.last_insert_rowid();
    patch.apply(&db, "habits", id, false)?;
    created(require(&db, "habits", "Habit", uid, id)?)
}

pub(super) fn habit_update(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let db = ctx.db();
    require(&db, "habits", "Habit", uid, id)?;
    let mut patch = Patch::default();
    habit_fields(ctx.body()?, &mut patch)?;
    patch.apply(&db, "habits", id, false)?;
    ok(require(&db, "habits", "Habit", uid, id)?)
}

pub(super) fn habit_delete(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let db = ctx.db();
    require(&db, "habits", "Habit", uid, ctx.params[0])?;
    db.execute("DELETE FROM habits WHERE id = ?1", params![ctx.params[0]])?;
    done()
}

/// {"day": "YYYY-MM-DD", "done": true|false}
pub(super) fn habit_check(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let id = ctx.params[0];
    let body = ctx.body()?;
    let day = clean_date(body.get("day"), "day")?.ok_or_else(|| bad("day is required"))?;
    let db = ctx.db();
    require(&db, "habits", "Habit", uid, id)?;
    if body.get("done").is_none_or(truthy) {
        db.execute("INSERT OR IGNORE INTO habit_checks (habit_id, day) VALUES (?1, ?2)", params![id, day])?;
    } else {
        db.execute("DELETE FROM habit_checks WHERE habit_id = ?1 AND day = ?2", params![id, day])?;
    }
    done()
}
