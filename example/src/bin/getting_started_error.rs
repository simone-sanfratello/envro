//! Getting-started Config with deliberately invalid values.
//!
//! ```bash
//! cd example && cargo run --bin getting_started_error
//! ```

use std::env;
use std::process;

use envro::{Envro, EnvroConfig, EnvroError};

#[derive(Envro, Debug)]
#[allow(dead_code)]
struct Config {
    #[envro(from = "APP_NAME", min_len = 1, max_len = 64)]
    app_name: String,

    #[envro(from = "APP_PORT", port)]
    app_port: u16,

    #[envro(from = "DATABASE_URL", min_len = 1, starts_with = "pg://")]
    database_url: String,

    #[envro(from = "DB_POOL_SIZE", positive_integer)]
    db_pool_size: i64,

    #[envro(from = "LOG_LEVEL", one_of("debug", "info", "warn", "error"))]
    log_level: String,

    #[envro(from = "FEATURE_METRICS", boolean)]
    feature_metrics: bool,
}

fn main() {
    let env_file = env::current_dir()
        .unwrap()
        .join(".env-getting-started-error");

    match Config::from_dotenv(&env_file) {
        Ok(config) => {
            eprintln!("expected validation failure, got: {config:?}");
            process::exit(2);
        }
        Err(err) => {
            eprintln!("{err}");
            if matches!(err, EnvroError::Validation { .. }) {
                process::exit(1);
            }
            process::exit(3);
        }
    }
}
