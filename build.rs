//! Embeds the built frontend (web/dist) into the binary, pre-compressed, so the server reads
//! nothing from disk and serves each asset with gzip and an ETag for almost no CPU or memory.
//!
//! The frontend is built with `npm run build` in web/. This script runs that build itself when
//! web/dist is missing or older than the sources, unless BROADSHEET_SKIP_WEB is set (the Docker
//! image builds the frontend in its own stage).

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;
use std::{env, fs};

fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, PathBuf)>) {
    for entry in fs::read_dir(dir).expect("read frontend dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            walk(&path, root, out);
        } else {
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.push((format!("/{rel}"), path));
        }
    }
}

/// The newest modification time of anything under `path`.
fn newest(path: &Path) -> Option<SystemTime> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_dir() {
        return meta.modified().ok();
    }
    fs::read_dir(path).ok()?.filter_map(|e| newest(&e.ok()?.path())).max()
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "svg" => "image/svg+xml",
        "json" | "webmanifest" => "application/json; charset=utf-8",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        _ => "application/octet-stream",
    }
}

// FNV-1a 64: a cheap content fingerprint for ETags.
fn fingerprint(data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("\"{hash:016x}\"")
}

fn npm(web: &Path, args: &[&str]) {
    let status = Command::new("npm").args(args).current_dir(web).status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("`npm {}` in web/ failed ({s})", args.join(" ")),
        Err(e) => panic!("could not run npm to build the frontend ({e}); install Node.js, or build web/ yourself and set BROADSHEET_SKIP_WEB=1"),
    }
}

fn build_frontend(web: &Path, dist: &Path) {
    let sources = ["src", "public", "index.html", "package.json", "vite.config.js"];
    for s in sources {
        println!("cargo:rerun-if-changed=web/{s}");
    }
    println!("cargo:rerun-if-env-changed=BROADSHEET_SKIP_WEB");
    if env::var_os("BROADSHEET_SKIP_WEB").is_some() {
        return;
    }
    let built = newest(&dist.join("index.html"));
    let changed = sources.iter().filter_map(|s| newest(&web.join(s))).max();
    if built.is_some() && built >= changed {
        return;
    }
    if !web.join("node_modules").exists() {
        npm(web, &["ci", "--no-audit", "--no-fund"]);
    }
    npm(web, &["run", "build"]);
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let web = manifest.join("web");
    let root = web.join("dist");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=web/dist");
    build_frontend(&web, &root);
    assert!(root.join("index.html").exists(), "web/dist/index.html is missing: run `npm --prefix web run build`");

    let mut files = Vec::new();
    walk(&root, &root, &mut files);
    files.sort();

    let mut code = String::from("pub static ASSETS: &[Asset] = &[\n");
    for (i, (url_path, file)) in files.iter().enumerate() {
        let data = fs::read(file).expect("read asset");
        // Fonts (woff2) and images are already compressed; gzip would only add bytes.
        let gz = if url_path.ends_with(".woff2") || url_path.ends_with(".woff") || url_path.ends_with(".png") {
            "None".to_string()
        } else {
            let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
            enc.write_all(&data).unwrap();
            let gz_path = out_dir.join(format!("asset_{i}.gz"));
            fs::write(&gz_path, enc.finish().unwrap()).unwrap();
            format!("Some(include_bytes!({:?}))", gz_path.to_string_lossy())
        };
        writeln!(
            code,
            "    Asset {{ path: {url_path:?}, ctype: {:?}, etag: {:?}, raw: include_bytes!({:?}), gz: {gz} }},",
            content_type(url_path),
            fingerprint(&data),
            file.to_string_lossy(),
        )
        .unwrap();
    }
    code.push_str("];\n");
    fs::write(out_dir.join("assets.rs"), code).unwrap();
}
