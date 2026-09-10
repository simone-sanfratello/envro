//! Compose a DATABASE_URI from validated parts via `${VAR}` substitution.
//!
//! Run from the `example/` directory:
//!
//! ```bash
//! cd example && cargo run
//! ```

use std::env;
use std::process;

use envro::{Envro, EnvroConfig};

#[derive(Debug, Envro)]
struct Config {
    #[envro(from = "PG_USER", min_len = 1, max_len = 63, alphanumeric)]
    pg_user: String,

    #[envro(from = "PG_PASS", min_len = 6)]
    pg_pass: String,

    #[envro(from = "PG_HOST", min_len = 1, max_len = 253)]
    pg_host: String,

    #[envro(from = "PG_PORT", port)]
    pg_port: u16,

    #[envro(from = "PG_DB", min_len = 1, alphanumeric)]
    pg_db: String,

    #[envro(from = "PG_SSLMODE", one_of("disable", "require", "verify-full"))]
    pg_sslmode: String,

    #[envro(from = "DATABASE_URI", min_len = 1, starts_with = "pg://")]
    database_uri: String,

    #[envro(from = "DB_POOL_SIZE", positive_integer)]
    db_pool_size: i64,
}

fn main() {
    let env_file = env::current_dir().unwrap().join(".env-sample");

    let config = match Config::from_dotenv(&env_file) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("validation failed: {err}");
            process::exit(1);
        }
    };

    println!("parts:");
    println!("  PG_USER={}", config.pg_user);
    println!("  PG_PASS={}", config.pg_pass);
    println!("  PG_HOST={}", config.pg_host);
    println!("  PG_PORT={}", config.pg_port);
    println!("  PG_DB={}", config.pg_db);
    println!("  PG_SSLMODE={}", config.pg_sslmode);
    println!();
    println!("DATABASE_URI={}", config.database_uri);
    println!("DB_POOL_SIZE={}", config.db_pool_size);
}
