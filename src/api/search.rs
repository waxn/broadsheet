//! Search across tasks, classes, pages and events.

use rusqlite::params;
use serde_json::{json, Value};

use super::*;

pub(super) fn search(ctx: &Ctx) -> ApiResult<Reply> {
    let q: String = ctx.query.get("q").map(|s| s.trim().chars().take(80).collect()).unwrap_or_default();
    if q.is_empty() {
        return ok(json!({ "todos": [], "classes": [], "pages": [], "events": [] }));
    }
    let like = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let uid = ctx.uid()?;
    let db = ctx.db();
    let todos = rows(
        &db,
        "SELECT t.*, c.name AS class_name, c.color AS class_color FROM todos t LEFT JOIN classes c ON c.id = t.class_id
         WHERE t.user_id = ?1 AND (t.title LIKE ?2 ESCAPE '\\' OR t.description LIKE ?2 ESCAPE '\\')
         ORDER BY t.done, t.id DESC LIMIT 10",
        params![uid, like],
    )?;
    let classes = rows(
        &db,
        "SELECT id, name, section, color, emoji FROM classes
         WHERE user_id = ?1 AND (name LIKE ?2 ESCAPE '\\' OR section LIKE ?2 ESCAPE '\\') LIMIT 6",
        params![uid, like],
    )?;
    // A short excerpt around the first match, for the result list.
    let mut pages = rows(
        &db,
        "SELECT p.id, p.title, p.icon, p.todo_id, p.class_id, p.text, p.body, COALESCE(t.title, c.name) AS owner_name
         FROM pages p LEFT JOIN todos t ON t.id = p.todo_id LEFT JOIN classes c ON c.id = p.class_id
         WHERE p.user_id = ?1 AND p.archived = 0
           AND (p.title LIKE ?2 ESCAPE '\\' OR p.text LIKE ?2 ESCAPE '\\' OR p.body LIKE ?2 ESCAPE '\\')
         ORDER BY p.updated_at DESC LIMIT 10",
        params![uid, like],
    )?;
    let needle = q.to_lowercase();
    for page in &mut pages {
        let text = page["text"].as_str().filter(|t| !t.is_empty()).or(page["body"].as_str()).unwrap_or("").to_string();
        let lower = text.to_lowercase();
        let excerpt = match lower.find(&needle) {
            // Lowercasing can change byte lengths, so fall back to the start when offsets do not line up.
            Some(at) if text.is_char_boundary(at) => {
                let start = text[..at].char_indices().rev().nth(40).map_or(0, |(i, _)| i);
                text[start..].chars().take(120).collect::<String>()
            }
            _ => text.chars().take(120).collect(),
        };
        let obj = page.as_object_mut().unwrap();
        obj.remove("text");
        obj.remove("body");
        obj.insert("excerpt".into(), Value::String(excerpt.split_whitespace().collect::<Vec<_>>().join(" ")));
    }
    let events = rows(
        &db,
        "SELECT id, title, start_at, end_at, all_day, color FROM events
         WHERE user_id = ?1 AND (title LIKE ?2 ESCAPE '\\' OR location LIKE ?2 ESCAPE '\\' OR notes LIKE ?2 ESCAPE '\\')
         ORDER BY start_at DESC LIMIT 8",
        params![uid, like],
    )?;
    ok(json!({ "todos": todos, "classes": classes, "pages": pages, "events": events }))
}
