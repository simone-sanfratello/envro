//! Age-encrypted secrets in `.env` (feature `encryption`).
//!
//! Follows [docs/encryption.md](../../docs/encryption.md):
//! - `ENVRO_AGE_IDENTITY_FILE` in `.env` (path to private key)
//! - `PG_PASS=Encrypted[AGE:b64:…]` + `#[envro(secret)]`
//! - CI can still inject plaintext `PG_PASS` and use `Config::from_env()`
//!
//! Setup (once):
//!
//! ```bash
//! mkdir -p ~/.config/envro
//! age-keygen -pq -o ~/.config/envro/example.key
//! # encrypt a password to the printed age1pq1… public key, then put
//! # Encrypted[AGE:b64:…] into example/.env-encryption as PG_PASS
//! ```
//!
//! Run from the `example/` directory (after implementation):
//!
//! ```bash
//! cd example && cargo run --bin encryption
//! # example/Cargo.toml already has: envro = { path = "../", features = ["encryption"] }
//! # apps: cargo add envro --features encryption
//! ```

use std::env;
use std::process;

use envro::{Envro, EnvroConfig};

#[derive(Debug, Envro)]
struct Config {
    #[envro(from = "PG_HOST", min_len = 1, max_len = 253)]
    pg_host: String,

    #[envro(from = "PG_PASS", secret, min_len = 6)]
    pg_pass: String,

    #[envro(from = "DATABASE_URI", starts_with = "pg://")]
    database_uri: String,
}

fn main() {
    let env_file = env::current_dir().unwrap().join(".env-encryption");

    let config = match Config::from_dotenv(&env_file) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("load/validation failed: {err}");
            process::exit(1);
        }
    };

    // Do not log real secrets in production — demo only.
    println!("PG_HOST={}", config.pg_host);
    println!("PG_PASS length={}", config.pg_pass.len());
    println!("DATABASE_URI={}", config.database_uri);
}
