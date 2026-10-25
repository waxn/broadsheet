//! Settings, read from environment variables (set through .env by Docker Compose).

#[derive(Clone, Debug)]
pub struct Config {
    pub db_path: String,
    pub allow_signup: bool,
    pub session_days: i64,
    pub cookie_secure: bool,
}

fn flag(name: &str, default: &str) -> bool {
    std::env::var(name).unwrap_or_else(|_| default.to_string()).to_lowercase() == "true"
}

impl Config {
    pub fn from_env() -> Config {
        Config {
            db_path: std::env::var("DATABASE_PATH").unwrap_or_else(|_| "/data/broadsheet.db".into()),
            allow_signup: flag("ALLOW_SIGNUP", "true"),
            session_days: std::env::var("SESSION_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(30),
            cookie_secure: flag("COOKIE_SECURE", "false"),
        }
    }
}
