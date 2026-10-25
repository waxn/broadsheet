//! Password hashing (PBKDF2-SHA256, same stored format as the Python version) and session tokens.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use sha2::{Digest, Sha256};

pub const SESSION_COOKIE: &str = "bs_session";
const PBKDF2_ITERATIONS: u32 = 600_000;
const MAX_ACCEPTED_ITERATIONS: u32 = 10_000_000;

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).expect("operating system random source");
    buf
}

fn derive(password: &str, salt: &[u8], iterations: u32, len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut out);
    out
}

/// "pbkdf2_sha256$<iterations>$<salt b64>$<hash b64>"
pub fn hash_password(password: &str) -> String {
    let salt = random_bytes::<16>();
    let digest = derive(password, &salt, PBKDF2_ITERATIONS, 32);
    format!("pbkdf2_sha256${PBKDF2_ITERATIONS}${}${}", STANDARD.encode(salt), STANDARD.encode(digest))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn verify_password(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    if parts.len() != 4 || parts[0] != "pbkdf2_sha256" {
        return false;
    }
    let (Ok(iterations), Ok(salt), Ok(expected)) =
        (parts[1].parse::<u32>(), STANDARD.decode(parts[2]), STANDARD.decode(parts[3]))
    else {
        return false;
    };
    if iterations == 0 || iterations > MAX_ACCEPTED_ITERATIONS || expected.is_empty() {
        return false;
    }
    constant_time_eq(&derive(password, &salt, iterations, expected.len()), &expected)
}

pub fn new_session_token() -> String {
    URL_SAFE_NO_PAD.encode(random_bytes::<32>())
}

/// Only the hash of a token is stored, so a leaked database does not leak live sessions.
pub fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
