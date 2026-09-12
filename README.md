# Envro

[![Crates.io](https://img.shields.io/crates/v/envro.svg)](https://crates.io/crates/envro)
[![docs.rs](https://img.shields.io/docsrs/envro)](https://docs.rs/envro)

Env vars for Rust: validate with a composable rule set, load `.env` into `std::env`, and optionally derive a typed `Config`.

## Features

- **Validate** env vars against a `Schema` of composable rules — works on any `HashMap`, a `.env` file, or the live process environment
- **Load** a `.env` into the process with an explicit override policy, or **parse** it to a map with no side effects
- **Derive** a typed `Config` with `#[derive(Envro)]` — field types are coerce targets; `#[envro(...)]` attrs are the rules; values still load at runtime
- **Defaults** for optional vars — `Field::default_value("…")` / `#[envro(default = "…")]` (required for `Option<T>`)
- **Compose** derived values with `${VAR}` substitution, then validate each part
- **Small `.env` dialect** — comments, quotes, multiline, duplicate keys rejected
- **Encryption** — optional `encryption` feature; `Encrypted[AGE:b64:…]` in `.env` via `ENVRO_AGE_IDENTITY_FILE` + `#[envro(secret)]`; CI can inject plaintext ([docs/encryption.md](docs/encryption.md))

## Getting started

```bash
cargo add envro
# with encrypted secrets in .env (age):
cargo add envro --features encryption
```

### Typed app config

Typical service setup: optional `.env` for local dev, then validate and coerce the process environment into a struct.

```rust
use std::env;
use envro::{load_dotenv_in_env_vars, Envro, EnvroConfig, EnvroError};

#[derive(Envro, Debug)]
struct Config {
    #[envro(from = "APP_NAME", min_len = 1, max_len = 64)]
    app_name: String,

    #[envro(from = "APP_PORT", port)]
    app_port: u16,

    #[envro(from = "DATABASE_URL", min_len = 1, starts_with = "pg://")]
    database_url: String,

    #[envro(from = "DB_POOL_SIZE", positive_integer)]
    db_pool_size: i64,

    #[envro(from = "LOG_LEVEL", default = "info", one_of("debug", "info", "warn", "error"))]
    log_level: String,

    #[envro(from = "FEATURE_METRICS", boolean)]
    feature_metrics: bool,
}

fn main() -> Result<(), EnvroError> {
    let env_file = env::current_dir()?.join(".env");
    if env_file.is_file() {
        load_dotenv_in_env_vars(&env_file, false)?;
    }

    let config = Config::from_env()?;
    println!("{config:?}");
    Ok(())
}
```

On failure, `EnvroError::Validation` lists every failing rule:

```rust
// example/src/bin/getting_started_error.rs — cargo run --bin getting_started_error
match Config::from_dotenv(&env_file) {
    Ok(config) => println!("{config:?}"),
    Err(err) => eprintln!("{err}"),
    // VALIDATION_ERROR APP_NAME[required]: ...; APP_PORT[port]: 70000 not in 1..=65535; ...
}
```

### Real-world: CD / process env

In CI/CD, inject process env vars as plaintext. `#[envro(secret)]` still applies — process plaintext is allowed; only a `.env` file requires `Encrypted[…]`. Compose with `${VAR}` via `Config::from_env()`:

```rust
use envro::{Envro, EnvroConfig};

#[derive(Envro, Debug)]
struct Config {
    #[envro(from = "PG_USER", min_len = 1, max_len = 63, alphanumeric)]
    pg_user: String,

    #[envro(from = "PG_PASS", secret, min_len = 6)]
    pg_pass: String,

    #[envro(from = "PG_HOST", min_len = 1, max_len = 253)]
    pg_host: String,

    #[envro(from = "PG_PORT", port)]
    pg_port: u16,

    #[envro(from = "PG_DB", min_len = 1, alphanumeric)]
    pg_db: String,

    #[envro(from = "PG_SSLMODE", one_of("disable", "require", "verify-full"))]
    pg_sslmode: String,

    // e.g. DATABASE_URI=pg://${PG_USER}:${PG_PASS}@${PG_HOST}:${PG_PORT}/${PG_DB}?sslmode=${PG_SSLMODE}
    #[envro(from = "DATABASE_URI", starts_with = "pg://")]
    database_uri: String,

    #[envro(from = "DB_POOL_SIZE", positive_integer)]
    db_pool_size: i64,
}

fn main() -> Result<(), envro::EnvroError> {
    // optional local .env — skipped in CD when vars are already injected
    let env_file = std::env::current_dir()?.join(".env");
    if env_file.is_file() {
        envro::load_dotenv_in_env_vars(&env_file, false)?;
    }

    let config = Config::from_env()?;
    println!("{}", config.database_uri);
    Ok(())
}
```

Runnable version: `cd example && cargo run --bin example`.

See [docs/dotenv-format.md](docs/dotenv-format.md) for `${VAR}` rules.

### Without derive

Same rules as a hand-written `Schema` when you only need validation (maps, tests, no struct):

```rust
use envro::*;

let schema = Schema::new()
    .field("APP_PORT", Field::required().port())
    .field(
        "LOG_LEVEL",
        Field::default_value("info").one_of(&["debug", "info", "warn", "error"]),
    );

validate_env(&schema)?;
// or: load_dotenv_validated(&path, &schema)?;  validate(&vars, &schema)?;
```

## Documentation

| Doc | Contents |
| --- | --- |
| [docs/api.md](docs/api.md) | Public **APIs and types** (functions, `Schema` / `Field`, derive) |
| [docs/validation.md](docs/validation.md) | Rule reference (`secret`, …), semantics, error inspection |
| [docs/dotenv-format.md](docs/dotenv-format.md) | `.env` dialect, `${VAR}`, encrypted values |
| [docs/encryption.md](docs/encryption.md) | Age secrets, identity file, PQ keys, Compose / CI |
| [docs/comparison.md](docs/comparison.md) | Comparison with dotenvy, dotenv-ng, and related crates |
| [docs.rs/envro](https://docs.rs/envro) | Generated rustdoc |

## Out of scope

- **No multi-file layering** — one path per call ([CUE on inheritance](https://cuelang.org/docs/concept/configuration-use-case/#inheritance-based-configuration-languages), [Angular LIFT Flat](https://angular.io/guide/styleguide#flat))
- **No macros that bake env values into the binary** — `#[derive(Envro)]` encodes types and rules only; values always load at runtime

## Best practices

- **Never commit `.env`** — add `.env` to `.gitignore`; commit `.env-sample` / docs with placeholder values only
- **Validate at the boundary** — load optional local `.env`, then `Config::from_env()` / `validate_env` so missing or bad vars fail fast
- **Mark secrets** — `#[envro(secret)]` / `Field::secret()` so plaintext secrets in a `.env` are rejected ([docs/encryption.md](docs/encryption.md))
- **Don't use `.env` to prod** — inject config at deploy time via CI/containers as process env ([Zero Trust: protect identities](https://learn.microsoft.com/en-us/entra/fundamentals/zero-trust-protect-identities))
- **One identity per project** — keep age private keys outside the repo (`ENVRO_AGE_IDENTITY_FILE`); never reuse one key across all apps
- **Prefer compose over duplication** — build URLs with `${VAR}` and validate parts + the composed value
- **Fail closed in prod** — required fields, tight `one_of` / length / format rules; defaults only where a safe fallback exists

## License

[MIT](LICENSE) © 2024-2026 Simone Sanfratello

