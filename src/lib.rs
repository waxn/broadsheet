//! Broadsheet server: HTTP + SQLite + embedded frontend, with almost no dependencies.

pub mod api;
pub mod assets;
pub mod auth;
pub mod config;
pub mod db;
pub mod http;

use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

pub use config::Config;

/// Shared by every connection. One SQLite connection behind a mutex is plenty for a personal app
/// and keeps memory low; slow work (password hashing) happens outside the lock.
pub struct State {
    pub cfg: Config,
    pub db: Mutex<Connection>,
}

impl State {
    pub fn open(cfg: Config) -> rusqlite::Result<State> {
        let conn = db::open(&cfg.db_path)?;
        Ok(State { cfg, db: Mutex::new(conn) })
    }
}

/// Starts serving on a background thread and returns the bound address (used by the tests).
pub fn spawn(cfg: Config, bind: &str) -> std::io::Result<SocketAddr> {
    let state = Arc::new(State::open(cfg).map_err(std::io::Error::other)?);
    let listener = TcpListener::bind(bind)?;
    let addr = listener.local_addr()?;
    std::thread::spawn(move || http::serve(listener, state));
    Ok(addr)
}

/// Serves on the current thread, forever.
pub fn run(cfg: Config, bind: &str) -> std::io::Result<()> {
    let state = Arc::new(State::open(cfg).map_err(std::io::Error::other)?);
    let listener = TcpListener::bind(bind)?;
    eprintln!("Broadsheet listening on http://{}", listener.local_addr()?);
    http::serve(listener, state);
    Ok(())
}
