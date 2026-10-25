//! End-to-end tests: a real server on an ephemeral port with a throwaway database.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};

use broadsheet::Config;
use serde_json::{json, Value};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Server {
    addr: SocketAddr,
    dir: std::path::PathBuf,
}

impl Server {
    fn start() -> Server {
        Server::start_with(true)
    }

    fn start_with(allow_signup: bool) -> Server {
        let dir = std::env::temp_dir().join(format!("broadsheet-test-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&dir).unwrap();
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

    let (status, cls, _) = s.request("POST", "/api/classes", Some(json!({"name": "Biology", "section": "P3", "color": "forest"})), Some(&cookie));
    assert_eq!(status, 201);
    assert_eq!(cls["color"], "forest");
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

    // Each to-do gets a default page; add a second one.
    let (_, pages, _) = s.request("GET", &format!("/api/pages?todo_id={id}"), None, Some(&cookie));
    assert_eq!(pages.as_array().unwrap().len(), 1);
    let (status, page, _) = s.request("POST", "/api/pages", Some(json!({"todo_id": id, "title": "Draft"})), Some(&cookie));
    assert_eq!(status, 201);
    assert_eq!(page["position"], 1);
    let (_, saved, _) = s.request("PATCH", &format!("/api/pages/{}", page["id"]), Some(json!({"body": "# Intro\n\nHello ✓"})), Some(&cookie));
    assert_eq!(saved["body"], "# Intro\n\nHello ✓");
    let (status, ..) = s.request("POST", "/api/pages", Some(json!({"todo_id": id, "class_id": cls["id"]})), Some(&cookie));
    assert_eq!(status, 400);

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
    let (_, pages, _) = s.request("GET", &format!("/api/pages?todo_id={}", todo["id"]), None, Some(&cookie));
    s.request("PATCH", &format!("/api/pages/{}", pages[0]["id"]), Some(json!({"body": "burette notes"})), Some(&cookie));
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
    let (_, hits, _) = s.request("GET", "/api/search?q=chem", None, Some(&cookie));
    assert_eq!(hits["classes"][0]["id"], cls["id"]);
    let (_, hits, _) = s.request("GET", "/api/search?q=", None, Some(&cookie));
    assert_eq!(hits, json!({"todos": [], "classes": [], "pages": []}));
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
    assert!(String::from_utf8_lossy(&body).contains("<title>Tasks</title>"));
    let h = |name: &str| headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone());
    assert_eq!(h("x-frame-options").as_deref(), Some("DENY"));
    assert!(h("content-security-policy").unwrap().contains("script-src 'self'"));
    assert!(h("content-encoding").is_none());
    let etag = h("etag").unwrap();

    let (status, headers, gz) = s.raw(b"GET /js/app.js HTTP/1.1\r\nAccept-Encoding: gzip, deflate\r\nConnection: close\r\n\r\n");
    assert_eq!(status, 200);
    assert!(headers.iter().any(|(k, v)| k == "content-encoding" && v == "gzip"));
    assert_eq!(&gz[..2], &[0x1f, 0x8b], "gzip magic bytes");
    let (_, raw_headers, raw) = s.raw(b"GET /js/app.js HTTP/1.1\r\nConnection: close\r\n\r\n");
    assert!(gz.len() < raw.len());
    assert!(raw_headers.iter().any(|(k, v)| k == "content-type" && v.starts_with("text/javascript")));

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
