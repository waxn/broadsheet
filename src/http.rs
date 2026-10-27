//! A small HTTP/1.1 server on std::net: one request per connection, one thread per connection.
//! No framework keeps memory use close to the bare minimum.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::api::{self, ApiRequest, Reply};
use crate::{assets, State};

const MAX_HEADER_LINES: usize = 100;
const MAX_LINE_BYTES: u64 = 8192;
const MAX_CONNECTIONS: usize = 128;
const THREAD_STACK_BYTES: usize = 512 * 1024;

const SECURITY_HEADERS: [(&str, &str); 4] = [
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "same-origin"),
    ("X-Frame-Options", "DENY"),
    (
        "Content-Security-Policy",
        "default-src 'self'; style-src 'self' 'unsafe-inline'; \
         font-src 'self'; img-src 'self' data: blob:; script-src 'self'; \
         frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
    ),
];

struct RawRequest {
    method: String,
    target: String,
    headers: HashMap<String, String>,
    /// None when the declared body was over the limit (it is not read).
    body: Option<Vec<u8>>,
}

fn read_line(reader: &mut impl BufRead) -> std::io::Result<Option<String>> {
    let mut buf = Vec::new();
    let n = reader.by_ref().take(MAX_LINE_BYTES).read_until(b'\n', &mut buf)?;
    if n == 0 {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&buf).trim_end_matches(['\r', '\n']).to_string()))
}

fn read_request(stream: &TcpStream) -> std::io::Result<Option<RawRequest>> {
    let mut reader = BufReader::new(stream);
    let Some(request_line) = read_line(&mut reader)? else { return Ok(None) };
    let mut parts = request_line.split(' ');
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("").to_string();
    if method.is_empty() || target.is_empty() {
        return Ok(None);
    }

    let mut headers = HashMap::new();
    for _ in 0..MAX_HEADER_LINES {
        match read_line(&mut reader)? {
            None => break,
            Some(line) if line.is_empty() => break,
            Some(line) => {
                if let Some((name, value)) = line.split_once(':') {
                    headers.insert(name.trim().to_lowercase(), value.trim().to_string());
                }
            }
        }
    }

    let length: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let path = target.split('?').next().unwrap_or("");
    let body = if length > api::body_limit(&method, path) {
        None
    } else {
        let mut buf = vec![0u8; length];
        reader.read_exact(&mut buf)?;
        Some(buf)
    };
    Ok(Some(RawRequest { method, target, headers, body }))
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        _ => "Internal Server Error",
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    ctype: &str,
    extra: &[(&str, String)],
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n",
        reason(status),
        body.len()
    );
    for (name, value) in SECURITY_HEADERS.iter().map(|(n, v)| (*n, v.to_string())).chain(extra.iter().map(|(n, v)| (*n, v.clone()))) {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn send_reply(stream: &mut TcpStream, reply: &Reply) -> std::io::Result<()> {
    if let Some(file) = &reply.file {
        let extra: Vec<(&str, String)> = file.headers.iter().map(|(n, v)| (*n, v.clone())).collect();
        return write_response(stream, reply.status, &file.ctype, &extra, &file.bytes, reply.status == 304);
    }
    let body = serde_json::to_vec(&reply.payload).unwrap_or_else(|_| b"{}".to_vec());
    let mut extra = vec![("Cache-Control", "no-store".to_string())];
    extra.extend(reply.cookies.iter().map(|c| ("Set-Cookie", c.clone())));
    write_response(stream, reply.status, "application/json; charset=utf-8", &extra, &body, false)
}

fn serve_static(stream: &mut TcpStream, req: &RawRequest, path: &str, head_only: bool) -> std::io::Result<()> {
    let path = if path.is_empty() || path == "/" { "/index.html" } else { path };
    // The app routes in the browser (/today, /page/12, ...): any path that is not a file and
    // looks like a route (no extension, no dot segments) gets the app's index page.
    let is_route = !path.contains('.') && !path.contains("//");
    let Some(asset) = assets::find(path).or_else(|| is_route.then(|| assets::find("/index.html")).flatten()) else {
        return write_response(stream, 404, "text/plain; charset=utf-8", &[], b"Not found", head_only);
    };
    // Built assets carry a content hash in their name and fonts rarely change, so the browser
    // keeps them; the index page revalidates with its ETag on each load (a 304 when unchanged).
    let cache = if path.starts_with("/assets/") {
        "public, max-age=31536000, immutable"
    } else if path.starts_with("/fonts/") {
        "public, max-age=604800"
    } else {
        "no-cache"
    };
    let mut extra = vec![
        ("Cache-Control", cache.to_string()),
        ("ETag", asset.etag.to_string()),
        ("Vary", "Accept-Encoding".to_string()),
    ];
    if req.headers.get("if-none-match").is_some_and(|v| v == asset.etag) {
        return write_response(stream, 304, asset.ctype, &extra, b"", true);
    }
    let gzip = req.headers.get("accept-encoding").is_some_and(|v| v.contains("gzip"));
    let body = match asset.gz {
        Some(gz) if gzip => {
            extra.push(("Content-Encoding", "gzip".to_string()));
            gz
        }
        _ => asset.raw,
    };
    write_response(stream, 200, asset.ctype, &extra, body, head_only)
}

fn handle_connection(mut stream: TcpStream, state: &State) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(15)));
    let _ = stream.set_nodelay(true);

    let req = match read_request(&stream) {
        Ok(Some(r)) => r,
        _ => return,
    };
    let (path, query) = req.target.split_once('?').unwrap_or((req.target.as_str(), ""));

    let result = if path.starts_with("/api/") {
        let api_req = ApiRequest {
            method: &req.method,
            path,
            query,
            headers: &req.headers,
            body: req.body.as_deref(),
        };
        let reply = catch_unwind(AssertUnwindSafe(|| api::handle(state, &api_req))).unwrap_or_else(|_| {
            eprintln!("panic while handling {} {}", req.method, path);
            api::error_reply(api::ApiError { status: 500, message: "Something broke on the server".into() })
        });
        send_reply(&mut stream, &reply)
    } else if req.method == "GET" || req.method == "HEAD" {
        serve_static(&mut stream, &req, path, req.method == "HEAD")
    } else {
        write_response(&mut stream, 405, "text/plain; charset=utf-8", &[], b"", false)
    };
    let _ = result; // the client may have gone away
}

struct ConnectionSlot(Arc<AtomicUsize>);

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn serve(listener: TcpListener, state: Arc<State>) {
    let active = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        if active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
            active.fetch_sub(1, Ordering::SeqCst);
            continue; // over capacity: drop the connection
        }
        let slot = ConnectionSlot(active.clone());
        let state = state.clone();
        let spawned = thread::Builder::new().stack_size(THREAD_STACK_BYTES).spawn(move || {
            let _slot = slot;
            handle_connection(stream, &state);
        });
        if spawned.is_err() {
            eprintln!("could not start a connection thread");
        }
    }
}
