//! End-to-end tests: a real server on an ephemeral port with a throwaway database.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};

use broadsheet::Config;
use serde_json::{json, Value};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn fresh_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("broadsheet-test-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Server {
    addr: SocketAddr,
    dir: std::path::PathBuf,
}

impl Server {
    fn start() -> Server {
        Server::start_with(true)
    }

    fn start_with(allow_signup: bool) -> Server {
        Server::start_in(fresh_dir(), allow_signup)
    }

    fn start_in(dir: std::path::PathBuf, allow_signup: bool) -> Server {
        let cfg = Config {
            db_path: dir.join("test.db").to_string_lossy().into_owned(),
            allow_signup,
            session_days: 30,
            cookie_secure: false,
        };
        let addr = broadsheet::spawn(cfg, "127.0.0.1:0").unwrap();
        Server { addr, dir }
    }

    fn raw(&self, request: &[u8]) -> (u16, Vec<(String, String)>, Vec<u8>) {
        let mut stream = TcpStream::connect(self.addr).unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
        stream.write_all(request).unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        let split = buf.windows(4).position(|w| w == b"\r\n\r\n").expect("response head");
        let head = String::from_utf8_lossy(&buf[..split]).to_string();
        let mut lines = head.split("\r\n");
        let status: u16 = lines.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
        let headers = lines
            .filter_map(|l| l.split_once(": "))
            .map(|(k, v)| (k.to_lowercase(), v.to_string()))
            .collect();
        (status, headers, buf[split + 4..].to_vec())
    }

    fn request(&self, method: &str, path: &str, body: Option<Value>, cookie: Option<&str>) -> (u16, Value, Option<String>) {
        let mut req = format!("{method} {path} HTTP/1.1\r\nHost: test\r\nConnection: close\r\n");
        if let Some(c) = cookie {
            req.push_str(&format!("Cookie: {c}\r\n"));
        }
        let payload = body.map(|b| b.to_string()).unwrap_or_default();
        if !payload.is_empty() {
            req.push_str(&format!("Content-Type: application/json\r\nContent-Length: {}\r\n", payload.len()));
        }
        req.push_str("\r\n");
        req.push_str(&payload);
        let (status, headers, body) = self.raw(req.as_bytes());
        let set_cookie = headers.iter().find(|(k, _)| k == "set-cookie").map(|(_, v)| v.clone());
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null), set_cookie)
    }

    fn signup(&self, username: &str) -> String {
        let (status, data, cookie) =
            self.request("POST", "/api/signup", Some(json!({"username": username, "password": "correct horse"})), None);
        assert_eq!(status, 201, "{data}");
        cookie.unwrap().split(';').next().unwrap().to_string()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn signup_login_logout() {
    let s = Server::start();
    let cookie = s.signup("alice_01");
    let (status, me, _) = s.request("GET", "/api/me", None, Some(&cookie));
    assert_eq!(status, 200);
    assert_eq!(me["username"], "alice_01");

    let (status, ..) = s.request("POST", "/api/signup", Some(json!({"username": "ALICE_01", "password": "another pass"})), None);
    assert_eq!(status, 409);

    let (status, ..) = s.request("POST", "/api/login", Some(json!({"username": "alice_01", "password": "wrong"})), None);
    assert_eq!(status, 401);
    let (status, ..) = s.request("POST", "/api/login", Some(json!({"username": "nobody", "password": "wrong"})), None);
    assert_eq!(status, 401);

    let (status, _, set_cookie) =
        s.request("POST", "/api/login", Some(json!({"username": "Alice_01", "password": "correct horse"})), None);
    assert_eq!(status, 200);
    let set_cookie = set_cookie.unwrap();
    assert!(set_cookie.contains("HttpOnly") && set_cookie.contains("SameSite=Lax"), "{set_cookie}");

    s.request("POST", "/api/logout", None, Some(&cookie));
    let (status, ..) = s.request("GET", "/api/me", None, Some(&cookie));
    assert_eq!(status, 401);
}

#[test]
fn signup_can_be_closed() {
    let s = Server::start_with(false);
    let (status, data, _) = s.request("POST", "/api/signup", Some(json!({"username": "late_comer", "password": "correct horse"})), None);
    assert_eq!(status, 403);
    assert_eq!(data["error"], "Sign-up is closed on this server");
}

#[test]
fn validation() {
    let s = Server::start();
    let (status, ..) = s.request("POST", "/api/signup", Some(json!({"username": "x", "password": "long enough"})), None);
    assert_eq!(status, 400);
    let (status, ..) = s.request("POST", "/api/signup", Some(json!({"username": "valid_name", "password": "short"})), None);
    assert_eq!(status, 400);
    // A body that is not JSON is rejected.
    let (status, ..) = s.raw(b"POST /api/signup HTTP/1.1\r\nContent-Type: text/plain\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello");
    assert_eq!(status, 400);
    // A JSON array is not an object.
    let (status, ..) = s.raw(b"POST /api/login HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]");
    assert_eq!(status, 400);

    let cookie = s.signup("valid_user");
    for bad in [json!({"title": "x", "due_date": "2026-13-40"}), json!({"title": "x", "due_date": "2026-02-30"}), json!({"title": "x", "priority": 9}), json!({"title": "x", "priority": true}), json!({"title": "   "}), json!({"title": 5})] {
        let (status, ..) = s.request("POST", "/api/todos", Some(bad.clone()), Some(&cookie));
        assert_eq!(status, 400, "{bad}");
    }
    let (status, ok, _) = s.request("POST", "/api/todos", Some(json!({"title": "leap day", "due_date": "2028-02-29"})), Some(&cookie));
    assert_eq!(status, 201);
    assert_eq!(ok["due_date"], "2028-02-29");
}

#[test]
fn todos_classes_and_pages() {
    let s = Server::start();
    let cookie = s.signup("bob_student");

    let (status, cls, _) = s.request("POST", "/api/classes", Some(json!({"name": "Biology", "section": "P3", "color": "green"})), Some(&cookie));
    assert_eq!(status, 201);
    assert_eq!(cls["color"], "green");
    let (status, ..) = s.request("POST", "/api/classes", Some(json!({"name": "Bad", "color": "pink"})), Some(&cookie));
    assert_eq!(status, 400);

    let (status, todo, _) = s.request(
        "POST",
        "/api/todos",
        Some(json!({"title": "Lab report", "due_date": "2026-11-02", "priority": 3, "class_id": cls["id"]})),
        Some(&cookie),
    );
    assert_eq!(status, 201);
    assert_eq!(todo["class_name"], "Biology");
    assert_eq!(todo["priority"], 3);
    let id = todo["id"].as_i64().unwrap();

    let (_, updated, _) = s.request("PATCH", &format!("/api/todos/{id}"), Some(json!({"done": true})), Some(&cookie));
    assert_eq!(updated["done"], 1);
    assert!(!updated["completed_at"].is_null());
    let (_, updated, _) = s.request("PATCH", &format!("/api/todos/{id}"), Some(json!({"done": false, "title": "Lab report 2"})), Some(&cookie));
    assert_eq!(updated["done"], 0);
    assert!(updated["completed_at"].is_null());
    assert_eq!(updated["title"], "Lab report 2");

    let (_, detail, _) = s.request("GET", &format!("/api/classes/{}", cls["id"]), None, Some(&cookie));
    assert_eq!(detail["todos"].as_array().unwrap().len(), 1);
    let (_, list, _) = s.request("GET", "/api/classes", None, Some(&cookie));
    assert_eq!(list[0]["task_count"], 1);
    assert_eq!(list[0]["open_count"], 1);

    // To-dos get pages on demand.
    let (_, pages, _) = s.request("GET", &format!("/api/pages?todo_id={id}"), None, Some(&cookie));
    assert_eq!(pages.as_array().unwrap().len(), 0);
    s.request("POST", "/api/pages", Some(json!({"todo_id": id, "title": "Outline"})), Some(&cookie));
    let (status, page, _) = s.request("POST", "/api/pages", Some(json!({"todo_id": id, "title": "Draft"})), Some(&cookie));
    assert_eq!(status, 201);
    assert_eq!(page["position"], 1);
    let (_, saved, _) = s.request("PATCH", &format!("/api/pages/{}", page["id"]), Some(json!({"body": "# Intro\n\nHello ✓"})), Some(&cookie));
    assert_eq!(saved["body"], "# Intro\n\nHello ✓");
    // Given both owners, a page belongs to the to-do.
    let (_, both, _) = s.request("POST", "/api/pages", Some(json!({"todo_id": id, "class_id": cls["id"]})), Some(&cookie));
    assert!(both["class_id"].is_null());

    let (_, class_pages, _) = s.request("GET", &format!("/api/pages?class_id={}", cls["id"]), None, Some(&cookie));
    assert_eq!(class_pages.as_array().unwrap().len(), 1);

    // Deleting a class keeps its to-dos but removes its document.
    let (status, ..) = s.request("DELETE", &format!("/api/classes/{}", cls["id"]), None, Some(&cookie));
    assert_eq!(status, 200);
    let (_, survivor, _) = s.request("GET", &format!("/api/todos/{id}"), None, Some(&cookie));
    assert!(survivor["class_id"].is_null());

    // Deleting a to-do removes its pages.
    s.request("DELETE", &format!("/api/todos/{id}"), None, Some(&cookie));
    let (status, ..) = s.request("GET", &format!("/api/pages/{}", page["id"]), None, Some(&cookie));
    assert_eq!(status, 404);
}

#[test]
fn users_cannot_see_each_other() {
    let s = Server::start();
    let owner = s.signup("carol_owner");
    let other = s.signup("dave_other");
    let (_, todo, _) = s.request("POST", "/api/todos", Some(json!({"title": "Secret"})), Some(&owner));
    let id = todo["id"].as_i64().unwrap();
    let (_, cls, _) = s.request("POST", "/api/classes", Some(json!({"name": "Private"})), Some(&owner));

    for (method, path, body) in [
        ("GET", format!("/api/todos/{id}"), None),
        ("PATCH", format!("/api/todos/{id}"), Some(json!({"title": "hacked"}))),
        ("DELETE", format!("/api/todos/{id}"), None),
        ("GET", format!("/api/pages?todo_id={id}"), None),
        ("GET", format!("/api/classes/{}", cls["id"]), None),
        ("PATCH", format!("/api/classes/{}", cls["id"]), Some(json!({"name": "hacked"}))),
    ] {
        let (status, ..) = s.request(method, &path, body, Some(&other));
        assert_eq!(status, 404, "{method} {path}");
    }
    // Cannot file your own to-do under someone else's class.
    let (status, ..) = s.request("POST", "/api/todos", Some(json!({"title": "x", "class_id": cls["id"]})), Some(&other));
    assert_eq!(status, 404);

    let (_, todos, _) = s.request("GET", "/api/todos", None, Some(&other));
    assert_eq!(todos, json!([]));
}

#[test]
fn search_is_scoped_and_escaped() {
    let s = Server::start();
    let cookie = s.signup("erin_search");
    let other = s.signup("frank_search");
    let (_, cls, _) = s.request("POST", "/api/classes", Some(json!({"name": "Chemistry"})), Some(&cookie));
    let (_, todo, _) = s.request("POST", "/api/todos", Some(json!({"title": "Titration lab 100%"})), Some(&cookie));
    let (_, page, _) = s.request("POST", "/api/pages", Some(json!({"todo_id": todo["id"]})), Some(&cookie));
    s.request("PATCH", &format!("/api/pages/{}", page["id"]), Some(json!({"text": "burette notes"})), Some(&cookie));
    s.request("POST", "/api/todos", Some(json!({"title": "Titration secret"})), Some(&other));

    let (_, hits, _) = s.request("GET", "/api/search?q=titration", None, Some(&cookie));
    assert_eq!(hits["todos"].as_array().unwrap().len(), 1);
    assert_eq!(hits["todos"][0]["title"], "Titration lab 100%");
    let (_, hits, _) = s.request("GET", "/api/search?q=100%25", None, Some(&cookie));
    assert_eq!(hits["todos"].as_array().unwrap().len(), 1);
    // A bare wildcard matches literally, not everything.
    let (_, hits, _) = s.request("GET", "/api/search?q=%25", None, Some(&cookie));
    assert_eq!(hits["todos"].as_array().unwrap().len(), 1);
    let (_, hits, _) = s.request("GET", "/api/search?q=burette", None, Some(&cookie));
    assert_eq!(hits["pages"].as_array().unwrap().len(), 1);
    assert_eq!(hits["pages"][0]["excerpt"], "burette notes");
    let (_, hits, _) = s.request("GET", "/api/search?q=chem", None, Some(&cookie));
    assert_eq!(hits["classes"][0]["id"], cls["id"]);
    let (_, hits, _) = s.request("GET", "/api/search?q=", None, Some(&cookie));
    assert_eq!(hits, json!({"todos": [], "classes": [], "pages": [], "events": []}));
    let (status, ..) = s.request("GET", "/api/search?q=x", None, None);
    assert_eq!(status, 401);
}

#[test]
fn routing_errors() {
    let s = Server::start();
    assert_eq!(s.request("GET", "/api/nope", None, None).0, 404);
    assert_eq!(s.request("GET", "/api/login", None, None).0, 405);
    assert_eq!(s.request("GET", "/api/todos", None, None).0, 401);
    assert_eq!(s.request("GET", "/api/todos/abc", None, None).0, 404);
}

#[test]
fn oversized_bodies_are_refused() {
    let s = Server::start();
    let (status, ..) = s.raw(b"POST /api/login HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: 5000000\r\nConnection: close\r\n\r\n");
    assert_eq!(status, 413);
}

#[test]
fn static_files_gzip_etag_and_headers() {
    let s = Server::start();
    let (status, headers, body) = s.raw(b"GET / HTTP/1.1\r\nHost: t\r\nConnection: close\r\n\r\n");
    assert_eq!(status, 200);
    assert!(String::from_utf8_lossy(&body).contains("<title>Broadsheet</title>"));
    let h = |name: &str| headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone());
    assert_eq!(h("x-frame-options").as_deref(), Some("DENY"));
    assert!(h("content-security-policy").unwrap().contains("script-src 'self'"));
    assert!(h("content-encoding").is_none());
    let etag = h("etag").unwrap();

    // The built script has a hashed name and is cached for good.
    let html = String::from_utf8_lossy(&body).to_string();
    let script = html.split("src=\"").nth(1).and_then(|r| r.split('"').next()).expect("script tag").to_string();
    assert!(script.starts_with("/assets/") && script.ends_with(".js"), "{script}");
    let (status, headers, gz) = s.raw(format!("GET {script} HTTP/1.1\r\nAccept-Encoding: gzip, deflate\r\nConnection: close\r\n\r\n").as_bytes());
    assert_eq!(status, 200);
    assert!(headers.iter().any(|(k, v)| k == "content-encoding" && v == "gzip"));
    assert!(headers.iter().any(|(k, v)| k == "cache-control" && v.contains("immutable")));
    assert_eq!(&gz[..2], &[0x1f, 0x8b], "gzip magic bytes");
    let (_, raw_headers, raw) = s.raw(format!("GET {script} HTTP/1.1\r\nConnection: close\r\n\r\n").as_bytes());
    assert!(gz.len() < raw.len());
    assert!(raw_headers.iter().any(|(k, v)| k == "content-type" && v.starts_with("text/javascript")));

    // App routes are answered with the app; missing files are still 404.
    let (status, _, routed) = s.raw(b"GET /page/12 HTTP/1.1\r\nConnection: close\r\n\r\n");
    assert_eq!(status, 200);
    assert!(String::from_utf8_lossy(&routed).contains("<title>Broadsheet</title>"));
    assert_eq!(s.raw(b"GET /assets/missing.js HTTP/1.1\r\nConnection: close\r\n\r\n").0, 404);

    let revalidate = format!("GET / HTTP/1.1\r\nIf-None-Match: {etag}\r\nConnection: close\r\n\r\n");
    let (status, _, body) = s.raw(revalidate.as_bytes());
    assert_eq!(status, 304);
    assert!(body.is_empty());

    // Fonts are served from this server: right type, never gzipped again, cached for a week,
    // and the CSP no longer allows any third-party origin.
    let (status, font_headers, font) = s.raw(b"GET /fonts/tinos-normal-400-latin.woff2 HTTP/1.1\r\nAccept-Encoding: gzip\r\nConnection: close\r\n\r\n");
    assert_eq!(status, 200);
    assert_eq!(&font[..4], b"wOF2");
    let fh = |name: &str| font_headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone());
    assert_eq!(fh("content-type").as_deref(), Some("font/woff2"));
    assert!(fh("content-encoding").is_none());
    assert_eq!(fh("cache-control").as_deref(), Some("public, max-age=604800"));
    let csp = h("content-security-policy").unwrap();
    assert!(!csp.contains("googleapis") && !csp.contains("gstatic"), "{csp}");
    assert!(csp.contains("font-src 'self'"));
    let page = String::from_utf8_lossy(&body);
    assert!(!page.contains("googleapis") && !page.contains("gstatic"));

    assert_eq!(s.raw(b"GET /../Cargo.toml HTTP/1.1\r\nConnection: close\r\n\r\n").0, 404);
    assert_eq!(s.raw(b"GET /src/main.rs HTTP/1.1\r\nConnection: close\r\n\r\n").0, 404);
    assert_eq!(s.raw(b"POST / HTTP/1.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").0, 405);
    let (status, _, body) = s.raw(b"HEAD / HTTP/1.1\r\nConnection: close\r\n\r\n");
    assert_eq!(status, 200);
    assert!(body.is_empty());
}

#[test]
fn sections_subtasks_and_reorder() {
    let s = Server::start();
    let c = s.signup("gina_tasks");
    let (_, cls, _) = s.request("POST", "/api/classes", Some(json!({"name": "History", "color": "teal"})), Some(&c));
    let (status, sec, _) = s.request("POST", "/api/sections", Some(json!({"name": "Essays", "class_id": cls["id"]})), Some(&c));
    assert_eq!(status, 201);

    // A task filed in a section takes the section's class.
    let (_, parent, _) = s.request("POST", "/api/todos", Some(json!({"title": "Essay", "section_id": sec["id"], "due_time": "15:00", "description": "1500 words"})), Some(&c));
    assert_eq!(parent["class_id"], cls["id"]);
    assert_eq!(parent["due_time"], "15:00");
    let (_, child, _) = s.request("POST", "/api/todos", Some(json!({"title": "Outline", "parent_id": parent["id"]})), Some(&c));
    assert_eq!(child["section_id"], sec["id"]);

    // No cycles.
    let (status, ..) = s.request("PATCH", &format!("/api/todos/{}", parent["id"]), Some(json!({"parent_id": child["id"]})), Some(&c));
    assert_eq!(status, 400);

    // Moving the parent to the Inbox carries the subtask along.
    s.request("PATCH", &format!("/api/todos/{}", parent["id"]), Some(json!({"class_id": null})), Some(&c));
    let (_, moved, _) = s.request("GET", &format!("/api/todos/{}", child["id"]), None, Some(&c));
    assert!(moved["class_id"].is_null() && moved["section_id"].is_null());
    let (_, detail, _) = s.request("GET", &format!("/api/todos/{}", parent["id"]), None, Some(&c));
    assert_eq!(detail["subtasks"].as_array().unwrap().len(), 1);

    // Completing the parent completes the subtask.
    let (_, res, _) = s.request("POST", &format!("/api/todos/{}/complete", parent["id"]), Some(json!({})), Some(&c));
    assert_eq!(res["recurred"], false);
    let (_, kid, _) = s.request("GET", &format!("/api/todos/{}", child["id"]), None, Some(&c));
    assert_eq!(kid["done"], 1);

    // Drag and drop: reposition and refile in one call.
    let (status, ..) = s.request(
        "POST",
        "/api/reorder",
        Some(json!({"kind": "todos", "items": [{"id": parent["id"], "position": 5, "section_id": sec["id"]}]})),
        Some(&c),
    );
    assert_eq!(status, 200);
    let (_, after, _) = s.request("GET", &format!("/api/todos/{}", parent["id"]), None, Some(&c));
    assert_eq!(after["position"], 5);
    assert_eq!(after["class_id"], cls["id"]);
    let (_, kid, _) = s.request("GET", &format!("/api/todos/{}", child["id"]), None, Some(&c));
    assert_eq!(kid["class_id"], cls["id"]);

    // Someone else's ids are refused.
    let other = s.signup("hank_other");
    let (status, ..) = s.request("POST", "/api/reorder", Some(json!({"kind": "todos", "items": [{"id": parent["id"], "position": 0}]})), Some(&other));
    assert_eq!(status, 404);
    let (status, ..) = s.request("POST", "/api/todos", Some(json!({"title": "x", "section_id": sec["id"]})), Some(&other));
    assert_eq!(status, 404);

    // Deleting a section deletes its tasks.
    s.request("DELETE", &format!("/api/sections/{}", sec["id"]), None, Some(&c));
    assert_eq!(s.request("GET", &format!("/api/todos/{}", parent["id"]), None, Some(&c)).0, 404);
}

#[test]
fn recurring_tasks_roll_forward() {
    let s = Server::start();
    let c = s.signup("ivy_repeats");
    let (status, ..) = s.request("POST", "/api/todos", Some(json!({"title": "x", "recurrence": {"freq": "hourly"}})), Some(&c));
    assert_eq!(status, 400);
    let (_, t, _) = s.request(
        "POST",
        "/api/todos",
        Some(json!({"title": "Gym", "due_date": "2026-10-26", "recurrence": {"freq": "week", "days": [0, 2], "text": "every mon, wed"},
                    "start_at": "2026-10-26T07:00", "end_at": "2026-10-26T08:00"})),
        Some(&c),
    );
    let id = t["id"].as_i64().unwrap();
    let (_, res, _) = s.request("POST", &format!("/api/todos/{id}/complete"), Some(json!({"today": "2026-10-26"})), Some(&c));
    assert_eq!(res["recurred"], true);
    assert_eq!(res["todo"]["due_date"], "2026-10-28");
    assert_eq!(res["todo"]["done"], 0);
    assert!(res["todo"]["start_at"].is_null());
    // Overdue: the next date after today, not after the old due date.
    let (_, res, _) = s.request("POST", &format!("/api/todos/{id}/complete"), Some(json!({"today": "2026-11-10"})), Some(&c));
    assert_eq!(res["todo"]["due_date"], "2026-11-11");
    let rule: Value = serde_json::from_str(res["todo"]["recurrence"].as_str().unwrap()).unwrap();
    assert_eq!(rule["text"], "every mon, wed");

    let (status, ..) = s.request("PATCH", &format!("/api/todos/{id}"), Some(json!({"start_at": "2026-10-26T09:00", "end_at": "2026-10-26T08:00"})), Some(&c));
    assert_eq!(status, 400);
    let (status, ..) = s.request("PATCH", &format!("/api/todos/{id}"), Some(json!({"due_time": "25:00"})), Some(&c));
    assert_eq!(status, 400);
}

#[test]
fn nested_pages_links_and_trash() {
    let s = Server::start();
    let c = s.signup("jules_pages");
    let (_, root, _) = s.request("POST", "/api/pages", Some(json!({"title": "Notebook", "icon": "📓"})), Some(&c));
    let (_, child, _) = s.request("POST", "/api/pages", Some(json!({"title": "Week 1", "parent_id": root["id"]})), Some(&c));
    let (_, grandchild, _) = s.request("POST", "/api/pages", Some(json!({"title": "Day 1", "parent_id": child["id"]})), Some(&c));

    let doc = json!({"type": "doc", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "see"}]}]});
    let (status, ..) = s.request(
        "PATCH",
        &format!("/api/pages/{}", grandchild["id"]),
        Some(json!({"content": doc, "text": "see", "links": [{"kind": "page", "id": root["id"]}]})),
        Some(&c),
    );
    assert_eq!(status, 200);
    let (_, got, _) = s.request("GET", &format!("/api/pages/{}", grandchild["id"]), None, Some(&c));
    assert_eq!(got["content"], doc);
    assert_eq!(got["path"].as_array().unwrap().iter().map(|p| p["title"].clone()).collect::<Vec<_>>(), vec![json!("Notebook"), json!("Week 1")]);
    let (_, r, _) = s.request("GET", &format!("/api/pages/{}", root["id"]), None, Some(&c));
    assert_eq!(r["backlinks"][0]["title"], "Day 1");
    assert_eq!(r["children"][0]["title"], "Week 1");

    // No moving a page inside its own subtree.
    let (status, ..) = s.request("PATCH", &format!("/api/pages/{}", root["id"]), Some(json!({"parent_id": grandchild["id"]})), Some(&c));
    assert_eq!(status, 400);
    let (status, ..) = s.request("PATCH", &format!("/api/pages/{}", root["id"]), Some(json!({"links": [{"kind": "spaceship", "id": 1}]})), Some(&c));
    assert_eq!(status, 400);

    // Trash takes the subtree; restore brings it back.
    s.request("PATCH", &format!("/api/pages/{}", child["id"]), Some(json!({"archived": true})), Some(&c));
    let (_, tree, _) = s.request("GET", "/api/pages", None, Some(&c));
    assert_eq!(tree.as_array().unwrap().len(), 1);
    let (_, trash, _) = s.request("GET", "/api/pages?archived=1", None, Some(&c));
    assert_eq!(trash.as_array().unwrap().len(), 2);
    s.request("PATCH", &format!("/api/pages/{}", child["id"]), Some(json!({"archived": false})), Some(&c));
    let (_, tree, _) = s.request("GET", "/api/pages", None, Some(&c));
    assert_eq!(tree.as_array().unwrap().len(), 3);

    let other = s.signup("kim_other");
    assert_eq!(s.request("GET", &format!("/api/pages/{}", root["id"]), None, Some(&other)).0, 404);
    let (status, ..) = s.request("POST", "/api/pages", Some(json!({"parent_id": root["id"]})), Some(&other));
    assert_eq!(status, 404);
}

#[test]
fn events_countdowns_and_habits() {
    let s = Server::start();
    let c = s.signup("lee_calendar");
    let (status, ev, _) = s.request(
        "POST",
        "/api/events",
        Some(json!({"title": "Chem lab", "start_at": "2026-10-27T09:00", "end_at": "2026-10-27T10:30", "color": "blue"})),
        Some(&c),
    );
    assert_eq!(status, 201, "{ev}");
    let (_, exam, _) = s.request("POST", "/api/events", Some(json!({"title": "Finals", "all_day": true, "start_at": "2026-12-10", "end_at": "2026-12-12", "countdown": true})), Some(&c));
    assert_eq!(exam["all_day"], 1);
    for bad in [
        json!({"title": "x", "start_at": "2026-10-27T09:00", "end_at": "2026-10-27T08:00"}),
        json!({"title": "x", "start_at": "2026-10-27"}),
        json!({"title": "x", "all_day": true, "start_at": "2026-10-27T09:00"}),
        json!({"title": "x"}),
        json!({"title": "x", "start_at": "2026-10-27T09:00", "color": "neon"}),
    ] {
        assert_eq!(s.request("POST", "/api/events", Some(bad.clone()), Some(&c)).0, 400, "{bad}");
    }

    let (_, week, _) = s.request("GET", "/api/events?from=2026-10-26&to=2026-11-01", None, Some(&c));
    assert_eq!(week.as_array().unwrap().len(), 1);
    // Multi-day events show on every day they cover.
    let (_, mid, _) = s.request("GET", "/api/events?from=2026-12-11&to=2026-12-11", None, Some(&c));
    assert_eq!(mid[0]["title"], "Finals");

    // Moving an event only needs the changed field.
    let (_, moved, _) = s.request("PATCH", &format!("/api/events/{}", ev["id"]), Some(json!({"start_at": "2026-10-27T08:00"})), Some(&c));
    assert_eq!(moved["start_at"], "2026-10-27T08:00");
    assert_eq!(moved["end_at"], "2026-10-27T10:30");

    let (_, cd, _) = s.request("GET", "/api/countdowns?today=2026-10-26", None, Some(&c));
    assert_eq!(cd[0]["title"], "Finals");
    let (_, cd, _) = s.request("GET", "/api/countdowns?today=2027-01-01", None, Some(&c));
    assert_eq!(cd, json!([]));

    let (_, h, _) = s.request("POST", "/api/habits", Some(json!({"name": "Read", "emoji": "📚"})), Some(&c));
    let path = format!("/api/habits/{}/check", h["id"]);
    s.request("POST", &path, Some(json!({"day": "2026-10-25"})), Some(&c));
    s.request("POST", &path, Some(json!({"day": "2026-10-26", "done": true})), Some(&c));
    s.request("POST", &path, Some(json!({"day": "2026-10-26", "done": true})), Some(&c));
    s.request("POST", &path, Some(json!({"day": "2026-10-25", "done": false})), Some(&c));
    let (_, habits, _) = s.request("GET", "/api/habits?from=2026-10-01&to=2026-10-31", None, Some(&c));
    assert_eq!(habits["checks"], json!([{"habit_id": h["id"], "day": "2026-10-26"}]));

    let other = s.signup("max_other");
    assert_eq!(s.request("POST", &path, Some(json!({"day": "2026-10-26"})), Some(&other)).0, 404);
    assert_eq!(s.request("DELETE", &format!("/api/events/{}", ev["id"]), None, Some(&other)).0, 404);
    let (_, none, _) = s.request("GET", "/api/events", None, Some(&other));
    assert_eq!(none, json!([]));
}

#[test]
fn file_uploads() {
    let s = Server::start();
    let c = s.signup("nora_files");
    let png = b"\x89PNG\r\n\x1a\nnot really a png";
    let mut req = format!(
        "POST /api/files HTTP/1.1\r\nCookie: {c}\r\nContent-Type: image/png\r\nX-Filename: diagram%20one.png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        png.len()
    )
    .into_bytes();
    req.extend_from_slice(png);
    let (status, _, body) = s.raw(&req);
    assert_eq!(status, 201);
    let file: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(file["name"], "diagram one.png");
    let url = file["url"].as_str().unwrap();

    let (status, headers, bytes) = s.raw(format!("GET {url} HTTP/1.1\r\nCookie: {c}\r\nConnection: close\r\n\r\n").as_bytes());
    assert_eq!(status, 200);
    assert_eq!(bytes, png);
    let h = |name: &str| headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()).unwrap_or_default();
    assert_eq!(h("content-type"), "image/png");
    assert!(h("content-disposition").starts_with("inline"));
    let etag = h("etag");
    let (status, _, bytes) = s.raw(format!("GET {url} HTTP/1.1\r\nCookie: {c}\r\nIf-None-Match: {etag}\r\nConnection: close\r\n\r\n").as_bytes());
    assert_eq!(status, 304);
    assert!(bytes.is_empty());

    // Other people cannot read it, and scriptable types are downloads, not pages.
    let other = s.signup("omar_other");
    assert_eq!(s.raw(format!("GET {url} HTTP/1.1\r\nCookie: {other}\r\nConnection: close\r\n\r\n").as_bytes()).0, 404);
    let svg = b"<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>";
    let mut req = format!("POST /api/files HTTP/1.1\r\nCookie: {c}\r\nContent-Type: image/svg+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", svg.len()).into_bytes();
    req.extend_from_slice(svg);
    let (_, _, body) = s.raw(&req);
    let svg_file: Value = serde_json::from_slice(&body).unwrap();
    let (_, headers, _) = s.raw(format!("GET {} HTTP/1.1\r\nCookie: {c}\r\nConnection: close\r\n\r\n", svg_file["url"].as_str().unwrap()).as_bytes());
    assert!(headers.iter().any(|(k, v)| k == "content-disposition" && v.starts_with("attachment")));

    // Uploads may be bigger than JSON bodies, but not unlimited.
    let (status, ..) = s.raw(format!("POST /api/files HTTP/1.1\r\nCookie: {c}\r\nContent-Length: 30000000\r\nConnection: close\r\n\r\n").as_bytes());
    assert_eq!(status, 413);
}

#[test]
fn bootstrap_has_everything() {
    let s = Server::start();
    let c = s.signup("pia_boot");
    s.request("POST", "/api/classes", Some(json!({"name": "Art"})), Some(&c));
    s.request("POST", "/api/todos", Some(json!({"title": "Sketch"})), Some(&c));
    let (status, data, _) = s.request("GET", "/api/bootstrap", None, Some(&c));
    assert_eq!(status, 200);
    assert_eq!(data["user"]["username"], "pia_boot");
    assert_eq!(data["classes"][0]["name"], "Art");
    assert!(data["classes"][0]["home_page_id"].is_i64());
    assert_eq!(data["todos"][0]["title"], "Sketch");
    assert_eq!(data["pages"].as_array().unwrap().len(), 1);
    assert_eq!(s.request("GET", "/api/bootstrap", None, None).0, 401);
}

/// A database written by version 1 (the original schema) is upgraded in place on open.
#[test]
fn migrates_version_1_databases() {
    let dir = fresh_dir();
    let path = dir.join("test.db");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT NOT NULL UNIQUE COLLATE NOCASE, password_hash TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')));
             CREATE TABLE sessions (token_hash TEXT PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, expires_at INTEGER NOT NULL);
             CREATE TABLE classes (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, name TEXT NOT NULL, section TEXT NOT NULL DEFAULT '', color TEXT NOT NULL DEFAULT 'oxblood', created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')));
             CREATE TABLE todos (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, class_id INTEGER REFERENCES classes(id) ON DELETE SET NULL, title TEXT NOT NULL, due_date TEXT, priority INTEGER NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 3), done INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)), created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')), completed_at TEXT);
             CREATE TABLE pages (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, todo_id INTEGER REFERENCES todos(id) ON DELETE CASCADE, class_id INTEGER REFERENCES classes(id) ON DELETE CASCADE, title TEXT NOT NULL DEFAULT 'Untitled', body TEXT NOT NULL DEFAULT '', position INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')), CHECK ((todo_id IS NULL) != (class_id IS NULL)));
             PRAGMA user_version = 1;",
        )
        .unwrap();
        let hash = broadsheet::auth::hash_password("correct horse");
        conn.execute("INSERT INTO users (id, username, password_hash) VALUES (1, 'old_timer', ?1)", [hash]).unwrap();
        conn.execute_batch(
            "INSERT INTO classes (id, user_id, name, color) VALUES (1, 1, 'Physics', 'navy');
             INSERT INTO todos (id, user_id, class_id, title, due_date, priority) VALUES (1, 1, 1, 'Problem set', '2026-11-01', 2);
             INSERT INTO todos (id, user_id, title, done) VALUES (2, 1, 'Old thing', 1);
             INSERT INTO pages (user_id, class_id, title, body) VALUES (1, 1, 'Notes', '# Mechanics');
             INSERT INTO pages (user_id, todo_id, title, body) VALUES (1, 1, 'Notes', '');
             INSERT INTO pages (user_id, todo_id, title, body) VALUES (1, 1, 'Working', '- [ ] q1');",
        )
        .unwrap();
    }
    let s = Server::start_in(dir, true);
    let (status, _, cookie) = s.request("POST", "/api/login", Some(json!({"username": "old_timer", "password": "correct horse"})), None);
    assert_eq!(status, 200, "old logins still work");
    let c = cookie.unwrap().split(';').next().unwrap().to_string();

    let (_, data, _) = s.request("GET", "/api/bootstrap", None, Some(&c));
    assert_eq!(data["classes"][0]["name"], "Physics");
    assert_eq!(data["classes"][0]["color"], "blue");
    let todo = data["todos"].as_array().unwrap().iter().find(|t| t["id"] == 1).unwrap().clone();
    assert_eq!(todo["title"], "Problem set");
    assert_eq!(todo["priority"], 2);
    // The empty default page went away; pages with writing survive with their markdown.
    let pages = data["pages"].as_array().unwrap();
    assert_eq!(pages.len(), 2);
    let class_page = pages.iter().find(|p| p["class_id"] == 1).unwrap();
    let (_, full, _) = s.request("GET", &format!("/api/pages/{}", class_page["id"]), None, Some(&c));
    assert_eq!(full["body"], "# Mechanics");
    assert!(full["content"].is_null());

    // New features work on the upgraded database.
    let (status, ..) = s.request("POST", "/api/pages", Some(json!({"title": "Free page"})), Some(&c));
    assert_eq!(status, 201);
    let (status, ..) = s.request("POST", "/api/sections", Some(json!({"name": "Labs", "class_id": 1})), Some(&c));
    assert_eq!(status, 201);
}
