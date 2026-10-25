use broadsheet::Config;

fn main() {
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".into());
    if let Err(err) = broadsheet::run(Config::from_env(), &format!("{host}:{port}")) {
        eprintln!("fatal: {err}");
        std::process::exit(1);
    }
}
