//! Compose a DATABASE_URI from validated parts via `${VAR}` substitution.
//!
//! Run from the `example/` directory:
//!
//! ```bash
//! cd example && cargo run
//! ```

use std::env;
use std::process;

use envro::*;

fn main() {
    let env_file = env::current_dir().unwrap().join(".env-sample");

    // Validate each piece on its own, then use the composed URI.
    let schema = Schema::new()
        .field(
            "PG_USER",
            Field::required().min_len(1).max_len(63).alphanumeric(),
        )
        .field("PG_PASS", Field::required().min_len(6))
        .field("PG_HOST", Field::required().min_len(1).max_len(253))
        .field("PG_PORT", Field::required().port())
        .field("PG_DB", Field::required().min_len(1).alphanumeric())
        .field(
            "PG_SSLMODE",
            Field::required().one_of(&["disable", "require", "verify-full"]),
        )
        .field(
            "DATABASE_URI",
            Field::required().min_len(1).starts_with("pg://"),
        )
        .field("DB_POOL_SIZE", Field::required().positive_integer());

    let vars = match load_dotenv_validated(&env_file, &schema) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("validation failed: {err}");
            process::exit(1);
        }
    };

    println!("parts:");
    for key in [
        "PG_USER",
        "PG_PASS",
        "PG_HOST",
        "PG_PORT",
        "PG_DB",
        "PG_SSLMODE",
    ] {
        println!("  {key}={}", vars.get(key).unwrap());
    }

    println!();
    println!(
        "DATABASE_URI={}",
        vars.get("DATABASE_URI").unwrap()
    );
    println!("DB_POOL_SIZE={}", vars.get("DB_POOL_SIZE").unwrap());
}
