//! Uploaded files (images and attachments in pages). Stored once per content hash next to the
//! database, and only ever served to the user who uploaded them.

use std::path::PathBuf;

use rusqlite::params;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::*;

pub(super) const MAX_UPLOAD_BYTES: usize = 25_000_000;

/// Types the browser may show in place; anything else downloads.
const INLINE: [&str; 7] = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/avif", "application/pdf", "text/plain"];

fn files_dir(ctx: &Ctx) -> PathBuf {
    let db = std::path::Path::new(&ctx.state.cfg.db_path);
    db.parent().filter(|p| !p.as_os_str().is_empty()).map_or_else(|| PathBuf::from("files"), |p| p.join("files"))
}

fn clean_mime(raw: Option<&String>) -> String {
    let mime = raw.map(|s| s.split(';').next().unwrap_or("").trim().to_lowercase()).unwrap_or_default();
    let valid = mime.split_once('/').is_some_and(|(a, b)| {
        let ok = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || b"+-.".contains(&c));
        ok(a) && ok(b)
    });
    if valid && mime.len() <= 100 {
        mime
    } else {
        "application/octet-stream".into()
    }
}

/// POST /api/files with the raw bytes as the body, the type in Content-Type and the
/// (percent-encoded) name in X-Filename.
pub(super) fn upload(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    if ctx.raw.is_empty() {
        return Err(bad("The file is empty"));
    }
    let name: String = ctx.headers.get("x-filename").map(|n| percent_decode(n)).unwrap_or_default();
    let name: String = name.chars().filter(|c| !c.is_control() && *c != '/' && *c != '\\').take(200).collect();
    let name = if name.trim().is_empty() { "file".to_string() } else { name.trim().to_string() };
    let mime = clean_mime(ctx.headers.get("content-type"));
    let sha: String = Sha256::digest(ctx.raw).iter().map(|b| format!("{b:02x}")).collect();

    let dir = files_dir(ctx);
    let path = dir.join(&sha);
    if !path.exists() {
        let write = || -> std::io::Result<()> {
            std::fs::create_dir_all(&dir)?;
            let tmp = dir.join(format!("{sha}.part"));
            std::fs::write(&tmp, ctx.raw)?;
            std::fs::rename(&tmp, &path)
        };
        write().map_err(|e| {
            eprintln!("could not store upload: {e}");
            ApiError::new(500, "Could not store the file")
        })?;
    }
    let db = ctx.db();
    db.execute(
        "INSERT INTO files (user_id, name, mime, size, sha256) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![uid, name, mime, ctx.raw.len() as i64, sha],
    )?;
    let id = db.last_insert_rowid();
    created(json!({ "id": id, "name": name, "mime": mime, "size": ctx.raw.len(), "url": format!("/api/files/{id}") }))
}

pub(super) fn download(ctx: &Ctx) -> ApiResult<Reply> {
    let uid = ctx.uid()?;
    let row = require(&ctx.db(), "files", "File", uid, ctx.params[0])?;
    let sha = row["sha256"].as_str().unwrap_or("").to_string();
    let etag = format!("\"{sha}\"");
    let mime = row["mime"].as_str().unwrap_or("application/octet-stream").to_string();
    let name = row["name"].as_str().unwrap_or("file");
    let encoded: String = name.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect();
    let disposition = if INLINE.contains(&mime.as_str()) { "inline" } else { "attachment" };
    let mut headers = vec![
        ("Cache-Control", "private, max-age=31536000, immutable".to_string()),
        ("ETag", etag.clone()),
        ("Content-Disposition", format!("{disposition}; filename*=UTF-8''{encoded}")),
    ];
    if ctx.headers.get("if-none-match").is_some_and(|v| *v == etag) {
        let mut reply = Reply::new(304, Value::Null);
        reply.file = Some(FileBody { ctype: mime, bytes: Vec::new(), headers });
        return Ok(reply);
    }
    let bytes = std::fs::read(files_dir(ctx).join(&sha)).map_err(|_| not_found("File"))?;
    headers.push(("Content-Security-Policy", "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; sandbox".into()));
    let mut reply = Reply::new(200, Value::Null);
    reply.file = Some(FileBody { ctype: mime, bytes, headers });
    Ok(reply)
}
