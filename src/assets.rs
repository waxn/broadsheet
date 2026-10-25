//! The frontend, embedded at build time (see build.rs).

pub struct Asset {
    pub path: &'static str,
    pub ctype: &'static str,
    pub etag: &'static str,
    pub raw: &'static [u8],
    /// Pre-compressed copy, absent for formats that are already compressed.
    pub gz: Option<&'static [u8]>,
}

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

pub fn find(path: &str) -> Option<&'static Asset> {
    ASSETS.iter().find(|a| a.path == path)
}
