//! Embeds the frontend into the binary, pre-compressed, so the server reads nothing from disk
//! and serves each asset with gzip and an ETag for almost no CPU or memory.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
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

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "svg" => "image/svg+xml",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
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

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("frontend");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=frontend");
    println!("cargo:rerun-if-changed=build.rs");

    let mut files = Vec::new();
    walk(&root, &root, &mut files);
    files.sort();

    let mut code = String::from("pub static ASSETS: &[Asset] = &[\n");
    for (i, (url_path, file)) in files.iter().enumerate() {
        let data = fs::read(file).expect("read asset");
        // Fonts (woff2) are already compressed; gzip would only add bytes.
        let gz = if url_path.ends_with(".woff2") {
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
