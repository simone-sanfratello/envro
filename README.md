# Envro

[![Crates.io](https://img.shields.io/crates/v/envro.svg)](https://crates.io/crates/envro)
[![docs.rs](https://img.shields.io/docsrs/envro)](https://docs.rs/envro)

Env vars for Rust: parse `.env` files, load them into `std::env`, and validate values with a composable rule set.

## Features

- **Parse** a `.env` file to a `HashMap` — no side effects on the process.
- **Load** a `.env` file into the process environment with an explicit override policy.
- **Validate** env vars against a `Schema` of composable rules. Validation is fully decoupled from loading: it works on any `HashMap`, on a `.env` file, or on the live process environment.
- **Small `.env` dialect**: `#` comments, empty values, double-quoted strings (with multi-line support), `=` inside values, `${VAR}` substitution (bare `$` literal; `\${VAR}` to skip), duplicate keys rejected.

## Getting started

```bash
cargo add envro
```

```rust
use std::env;
use envro::*;

fn main() {
    let env_file = env::current_dir().unwrap().join(".env");

    // parse only
    let vars = load_dotenv(&env_file).unwrap();

    // parse and set process env (keep existing non-empty values)
    load_dotenv_in_env_vars(&env_file, false).unwrap();

    println!("{vars:#?}");
}
```

## Loading `.env` files

### Types

- `EnvroVars` — alias for `HashMap<String, String>`
- `EnvroError` — errors from envro
  - `EnvroError::File` — cannot read the file
  - `EnvroError::Parse` — invalid line (missing `=`, empty name, unclosed quote, duplicate key, …)
  - `EnvroError::Validation` — validation issues (see [Validation](#validation))

### `load_dotenv(path) -> Result<EnvroVars, EnvroError>`

Parses a `.env` file and returns the variables as a map. Does **not** change process environment variables. Duplicate keys in the file are rejected.

```rust
use std::env;
use envro::*;

let env_file = env::current_dir().unwrap().join(".env");
let vars = load_dotenv(&env_file)?;

assert_eq!(vars.get("DB_POOL_SIZE"), Some(&"32".to_string()));
```

### `load_dotenv_in_env_vars(path, override_existing) -> Result<(), EnvroError>`

Parses the file and sets process environment variables.

| `override_existing` | Behavior |
| --- | --- |
| `false` | Keep existing **non-empty** process values; set unset or empty ones from the file |
| `true` | Always set values from the file, overwriting existing process values |

Keep existing values:

```rust
use std::env;
use envro::*;

env::set_var("DB_POOL_SIZE", "8");

let env_file = env::current_dir().unwrap().join(".env");
load_dotenv_in_env_vars(&env_file, false)?;

// process value wins when already set and non-empty
assert_eq!(env::var("DB_POOL_SIZE").unwrap(), "8");
```

Override existing values:

```rust
use std::env;
use envro::*;

env::set_var("DB_POOL_SIZE", "8");

let env_file = env::current_dir().unwrap().join(".env");
load_dotenv_in_env_vars(&env_file, true)?;

// file value wins
assert_eq!(env::var("DB_POOL_SIZE").unwrap(), "32");
```

## Validation

Envro validates env vars with a small, composable rule set. Validation is
**opt-in** and **decoupled from loading**: the same `Schema` works on any
`HashMap<String, String>`, on a `.env` file, or on the live process
environment. Envro collects every failing rule and returns them all at once.

### At a glance

- Build a `Schema` of `Field` specs, one per env var you care about.
- Every `Field` starts as `Field::required()` or `Field::optional()`, then chains rules.
- Envro collects **every** failing rule across every field before returning a single `EnvroError::Validation`.
- Validation is decoupled from where the vars come from: pass a `HashMap`, load from a `.env` file, or read directly from the process environment.

### Sources

Envro exposes three entry points that all share the same `Schema`:

**1. Any `HashMap<String, String>` — `validate(&vars, &schema)`**

Use when your env vars come from a CLI, a config service, test fixtures, or
already-loaded values:

```rust
use envro::*;

let mut vars = EnvroVars::new();
vars.insert("APP_NAME".into(), "envro".into());
vars.insert("PORT".into(),     "8080".into());

let schema = Schema::new()
    .field("APP_NAME", Field::required().min_len(3).max_len(64))
    .field("PORT",     Field::required().port());

validate(&vars, &schema)?;
```

**2. From a `.env` file — `load_dotenv_validated(path, &schema)`**

One-shot helper: parse the file, then validate:

```rust
use envro::*;

let env_file = std::env::current_dir().unwrap().join(".env");
let vars = load_dotenv_validated(&env_file, &schema)?;
```

Equivalent to `let vars = load_dotenv(&env_file)?; validate(&vars, &schema)?;`.

**3. Directly on the process environment — `validate_env(&schema)`**

Use when env vars are already set (shell exports, container runtime, systemd
unit, `env::set_var`, tests):

```rust
use envro::*;

// vars are already in the process environment
validate_env(&schema)?;
```

For every key in the schema, `validate_env` reads `std::env::var(key)` and
applies the field's rules. Missing/errored reads count as "not present" — same
semantics as an absent map key.

### Types and functions

- `Schema` — collection of `(name, Field)` specs
- `Field` — a required/optional field with a chain of rules
- `ValidationIssue` — one failure: `{ key: String, rule: &'static str, reason: String }`
- `EnvroError::Validation { errors: Vec<ValidationIssue> }` — carries every collected failure
- `validate(&EnvroVars, &Schema) -> Result<(), EnvroError>` — validate any map
- `validate_env(&Schema) -> Result<(), EnvroError>` — validate the process environment
- `load_dotenv_validated(&Path, &Schema) -> Result<EnvroVars, EnvroError>` — load a `.env` and validate in one call

### Semantics

| Situation | Behavior |
| --- | --- |
| Key missing from the map | `required` fails; `optional` skips remaining rules |
| Key present with `""`     | Same as missing (empty is treated as absent) |
| Key present with a value  | Every rule on the field is checked; all failures are collected |
| Key in the map but not in the schema | Ignored |
| Multiple failures on one field    | All collected |
| Multiple failing fields           | All collected in one `EnvroError::Validation` |
| Length rules (`min_len`, `max_len`, `exact_len`) | Count Unicode scalars, not bytes |

### Presence

Every field starts here. It is the only rule that is **not** chained.

```rust
Field::required()                 // must be present and non-empty
Field::optional().min_len(3)      // if present, must be >= 3 chars
```

`.env` view:

```env
# required  -> fails
APP_NAME=

# required  -> passes
APP_NAME=envro

# optional  -> passes (missing entirely, other rules skipped)
```

### Rule reference

Each entry below shows a schema snippet and one `.env` value that passes and
one that fails.

#### Length / string shape

- `min_len(n)` — at least `n` characters
  ```rust
  Field::required().min_len(3)
  ```
  ```env
  APP_NAME=envro   # ok
  APP_NAME=x       # fail: min_len
  ```

- `max_len(n)` — at most `n` characters
  ```rust
  Field::required().max_len(8)
  ```
  ```env
  SLUG=envro        # ok
  SLUG=very-long-x  # fail: max_len
  ```

- `exact_len(n)` — exactly `n` characters
  ```rust
  Field::required().exact_len(6)
  ```
  ```env
  COLOR=ff00aa   # ok
  COLOR=ff00     # fail: exact_len
  ```

- `alpha()` — only alphabetic characters
  ```rust
  Field::required().alpha()
  ```
  ```env
  REGION=eu      # ok
  REGION=eu1     # fail: alpha
  ```

- `alphanumeric()` — only letters and digits
  ```rust
  Field::required().alphanumeric()
  ```
  ```env
  BUILD=abc123   # ok
  BUILD=abc-123  # fail: alphanumeric
  ```

- `digits()` — only ASCII digits `0-9`
  ```rust
  Field::required().digits()
  ```
  ```env
  RETRY_COUNT=42   # ok
  RETRY_COUNT=42a  # fail: digits
  ```

- `ascii()` — only ASCII characters
  ```rust
  Field::required().ascii()
  ```
  ```env
  USER=alice     # ok
  USER=alicé     # fail: ascii
  ```

- `lowercase()` — no uppercase characters
  ```rust
  Field::required().lowercase()
  ```
  ```env
  BUCKET=media   # ok
  BUCKET=Media   # fail: lowercase
  ```

- `uppercase()` — no lowercase characters
  ```rust
  Field::required().uppercase()
  ```
  ```env
  REGION_CODE=EU    # ok
  REGION_CODE=Eu    # fail: uppercase
  ```

- `starts_with(s)` — must start with `s`
  ```rust
  Field::required().starts_with("pk_")
  ```
  ```env
  STRIPE_KEY=pk_live_123   # ok
  STRIPE_KEY=sk_live_123   # fail: starts_with
  ```

- `ends_with(s)` — must end with `s`
  ```rust
  Field::required().ends_with(".env")
  ```
  ```env
  CONFIG=prod.env   # ok
  CONFIG=prod.yaml  # fail: ends_with
  ```

- `contains(s)` — must contain `s`
  ```rust
  Field::required().contains("://")
  ```
  ```env
  DB_URL=pg://x   # ok
  DB_URL=no-url   # fail: contains
  ```

- `one_of(&[..])` — must equal one of the options
  ```rust
  Field::required().one_of(&["debug", "info", "warn", "error"])
  ```
  ```env
  LOG_LEVEL=info    # ok
  LOG_LEVEL=trace   # fail: one_of
  ```

- `not_one_of(&[..])` — must not equal any of the options
  ```rust
  Field::required().not_one_of(&["root", "admin"])
  ```
  ```env
  APP_USER=envro   # ok
  APP_USER=root    # fail: not_one_of
  ```

#### Numbers

- `integer()` — parses as `i64`
  ```rust
  Field::required().integer()
  ```
  ```env
  OFFSET=-3     # ok
  OFFSET=1.5    # fail: integer
  ```

- `positive_integer()` — `i64` and `> 0`
  ```rust
  Field::required().positive_integer()
  ```
  ```env
  WORKERS=4     # ok
  WORKERS=0     # fail: positive_integer
  ```

- `non_negative_integer()` — `i64` and `>= 0`
  ```rust
  Field::required().non_negative_integer()
  ```
  ```env
  RETRIES=0     # ok
  RETRIES=-1    # fail: non_negative_integer
  ```

- `float()` — parses as `f64`
  ```rust
  Field::required().float()
  ```
  ```env
  RATIO=0.75   # ok
  RATIO=abc    # fail: float
  ```

- `positive_float()` — `f64` and `> 0.0`
  ```rust
  Field::required().positive_float()
  ```
  ```env
  RATE=1.5     # ok
  RATE=0.0     # fail: positive_float
  ```

- `non_negative_float()` — `f64` and `>= 0.0`
  ```rust
  Field::required().non_negative_float()
  ```
  ```env
  RATE=0.0     # ok
  RATE=-0.5    # fail: non_negative_float
  ```

- `int_range(min, max)` — inclusive integer bounds
  ```rust
  Field::required().int_range(1, 100)
  ```
  ```env
  PERCENT=50    # ok
  PERCENT=150   # fail: int_range
  ```

- `float_range(min, max)` — inclusive float bounds
  ```rust
  Field::required().float_range(0.0, 1.0)
  ```
  ```env
  SAMPLE_RATE=0.25   # ok
  SAMPLE_RATE=2.0    # fail: float_range
  ```

- `port()` — integer in `1..=65535`
  ```rust
  Field::required().port()
  ```
  ```env
  PORT=8080    # ok
  PORT=70000   # fail: port
  ```

#### Boolean

- `boolean()` — one of `true` / `false` / `1` / `0` / `yes` / `no` (case-insensitive)
  ```rust
  Field::required().boolean()
  ```
  ```env
  FEATURE_X=true    # ok
  FEATURE_X=YES     # ok (case-insensitive)
  FEATURE_X=on      # fail: boolean (not accepted)
  ```

#### Formats (best-effort, std-only)

- `email()` — `local@domain`, no whitespace, both sides non-empty. Practical, not RFC 5322.
  ```rust
  Field::optional().email()
  ```
  ```env
  ADMIN_EMAIL=ops@example.com   # ok
  ADMIN_EMAIL=ops@              # fail: email (empty domain)
  ```

- `url()` — starts with `http://` or `https://`, remainder non-empty and whitespace-free
  ```rust
  Field::required().url()
  ```
  ```env
  WEBHOOK=https://example.com/hook   # ok
  WEBHOOK=ftp://x                    # fail: url (bad scheme)
  ```

- `uuid()` — canonical 8-4-4-4-12 hex form (dashes at positions 8, 13, 18, 23)
  ```rust
  Field::required().uuid()
  ```
  ```env
  TENANT_ID=550e8400-e29b-41d4-a716-446655440000   # ok
  TENANT_ID=not-a-uuid                             # fail: uuid
  ```

- `ipv4()` — parses via `std::net::Ipv4Addr`
  ```rust
  Field::required().ipv4()
  ```
  ```env
  BIND=127.0.0.1     # ok
  BIND=127.0.0.256   # fail: ipv4
  ```

- `ip()` — parses via `std::net::IpAddr` (v4 or v6)
  ```rust
  Field::required().ip()
  ```
  ```env
  BIND=::1           # ok
  BIND=not-an-ip     # fail: ip
  ```

- `hex()` — optional `0x` / `0X` prefix, rest must be non-empty hex digits
  ```rust
  Field::required().hex()
  ```
  ```env
  SECRET=0xDEADBEEF   # ok
  SECRET=0xZZ         # fail: hex
  ```

#### Lists

`list(delim, item)` splits the value on `delim`, trims each part, and applies
`item`'s rules to every element. Failing elements report the key as `KEY[i]`
where `i` is the 0-based position. `min_items(n)` and `max_items(n)` chain
after `list(..)` to bound the number of elements.

- required items, non-empty parts required:
  ```rust
  Field::required().list(',', Field::required().min_len(1))
  ```
  ```env
  TAGS=alpha,beta,gamma   # ok
  TAGS=alpha,,gamma       # fail: TAGS[1] required
  ```

- optional items, empty parts skipped:
  ```rust
  Field::required().list(',', Field::optional().min_len(2))
  ```
  ```env
  T=aa,,cc     # ok (empty middle skipped)
  T=a,bb,cc    # fail: T[0] min_len
  ```

- CSV of positive integers, with size bounds:
  ```rust
  Field::required()
      .list(',', Field::required().positive_integer())
      .min_items(1)
      .max_items(5)
  ```
  ```env
  PORTS=80,443,8080     # ok
  PORTS=                # fail: PORTS required (list rules skipped when empty)
  PORTS=1,2,3,4,5,6     # fail: max_items
  ```

### Inspecting errors

`EnvroError::Validation` carries the full list of `ValidationIssue`s, and
`Display` joins them under a `VALIDATION_ERROR ...` prefix:

```rust
match load_dotenv_validated(&env_file, &schema) {
    Ok(vars) => { /* use vars */ }
    Err(EnvroError::Validation { errors }) => {
        for issue in errors {
            eprintln!("{}[{}]: {}", issue.key, issue.rule, issue.reason);
        }
    }
    Err(other) => return Err(other),
}
```

Example message:

```
VALIDATION_ERROR PORT[port]: 70000 not in 1..=65535; ADMIN_EMAIL[email]: contains whitespace
```

### Not in v1

Intentionally out of scope for the current rule set:

- regex / arbitrary predicates
- filesystem path existence
- JSON schema, nested maps

## `.env` format

Small, explicit dialect. Values may contain `=`. Duplicate keys are a hard
error. Only `${VAR}` is expanded (from other keys in the same file, then the
process environment). Bare `$` is always literal. Use `\${VAR}` to keep the
braced form without replacement. Unknown, empty, or invalid `${…}` becomes an
empty string.

Example file:

```env
# comments are ignored
HOST=db.example.com
DB_CONNECTION_STRING=pg://user:pass@${HOST}/mydb
DB_POOL_SIZE=32
EMPTY=
QUOTED="value with spaces"
ESCAPED="say \"hello\""
WITH_EQUALS=host=localhost user=admin
LITERAL=\${HOST}
HASH=$2a$10$abc
```

### Variable substitution

| Form | Behavior |
| --- | --- |
| `${NAME}` | Replaced from other keys in the same file (any order), else from the process environment |
| `${}` | Replaced with `""` |
| `\${}` / `\${NAME}` | Literal `${}` / `${NAME}` (skip replacement) |
| Unknown / invalid `${…}` | Replaced with `""` |
| `$NAME` / `$2a$…` | Always literal — bare `$` is a normal character |

`NAME` must match `[A-Za-z_][A-Za-z0-9_]*`. Expansion is order-independent:
all keys are parsed first, then `${VAR}` refs are resolved across the file
(and the process env) in as many passes as needed. Circular references
resolve to empty strings. There is no `${NAME:-default}` syntax.

### Compose values, validate the parts

Store each knob as its own env var, compose derived values with `${VAR}`, and
validate every part with a `Schema`. That keeps rules close to the data
(length, port, allow-list, …) instead of parsing a blob at runtime.
Definition order does not matter.

**Example — Postgres URI from validated parts**

`.env`:

```env
PG_USER=app
PG_PASS=secret
PG_HOST=db.example.com
PG_PORT=5432
PG_DB=mydb
PG_SSLMODE=require

DATABASE_URI=pg://${PG_USER}:${PG_PASS}@${PG_HOST}:${PG_PORT}/${PG_DB}?sslmode=${PG_SSLMODE}
```

After load, `DATABASE_URI` is
`pg://app:secret@db.example.com:5432/mydb?sslmode=require`.

```rust
use envro::*;

let schema = Schema::new()
    .field("PG_USER", Field::required().min_len(1).max_len(63).alphanumeric())
    .field("PG_PASS", Field::required().min_len(6))
    .field("PG_HOST", Field::required().min_len(1).max_len(253))
    .field("PG_PORT", Field::required().port())
    .field("PG_DB",   Field::required().min_len(1).alphanumeric())
    .field("PG_SSLMODE", Field::required().one_of(&["disable", "require", "verify-full"]))
    .field(
        "DATABASE_URI",
        Field::required().min_len(1).starts_with("pg://"),
    );

let env_file = std::env::current_dir().unwrap().join(".env");
let vars = load_dotenv_validated(&env_file, &schema)?;

let uri = vars.get("DATABASE_URI").unwrap();
```

See `example/` for a runnable version of this pattern.

### Valid rows

| Row | Parses to | Notes |
| --- | --- | --- |
| `# any text`           | *(skipped)*                    | Full-line comment |
| *(empty line)*         | *(skipped)*                    | Blank lines are ignored |
| `NAME=envro`           | `NAME` = `envro`               | Basic `KEY=value` |
| `EMPTY=`               | `EMPTY` = `""`                 | Empty value, no quotes |
| `EMPTY=""`             | `EMPTY` = `""`                 | Empty quoted value |
| `QUOTED="a b"`         | `QUOTED` = `a b`               | Double-quoted value |
| `ESCAPED="say \"hi\""` | `ESCAPED` = `say "hi"`         | `\"` escapes an inner quote |
| `DSN=host=db user=admin` | `DSN` = `host=db user=admin` | `=` allowed inside value |
| `HOST=h` / `URL=${HOST}` (any order) | `URL` = `h` | Order-independent `${VAR}` |
| `A=abc${B}` / `B=123` | `A` = `abc123` | Forward refs resolve |
| `BARE=$HOST`           | `BARE` = `$HOST`               | Bare `$` never expanded |
| `LIT=\${HOST}`         | `LIT` = `${HOST}`              | `\${…}` skips replacement |
| `X=${}`                | `X` = `""`                     | Empty braces → empty |
| `X=\${}`               | `X` = `${}`                    | `\${}` skips replacement |
| `X=${MISSING}`         | `X` = `""`                     | Unknown `${VAR}` → empty |
| `X=${1}`               | `X` = `""`                     | Invalid name → empty |
| `HASH=$2a$10$abc`      | `HASH` = `$2a$10$abc`          | `$` allowed as a normal char |
| `URL="pg://u:p@h/db"`  | `URL` = `pg://u:p@h/db`        | Any chars fine inside quotes |
| `KEY="line1`<br/>`line2"` | `KEY` = `line1\nline2`      | Multi-line quoted value — newlines preserved |

### Multi-line values

Double-quoted values may span multiple physical lines. When a value opens
with `"` and does not close on the same line, envro keeps reading lines
(joining them with `\n`) until it finds a line ending with an unescaped `"`.
Lines inside the quotes are taken **literally** — blank lines and `#` at the
start of a line are part of the value, not comments. CRLF endings are
normalized to `\n` inside the value.

```env
PEM="-----BEGIN PRIVATE KEY-----
MIIBVwIBADANBgkqhkiG9w0BAQEFAA...
-----END PRIVATE KEY-----"
```

Escapes inside quotes: `\"` — a literal `"`; a trailing `\"` on a line therefore
does **not** close the value.

### Invalid rows

| Row | Error |
| --- | --- |
| `NAME value`      | `PARSE_ERROR ... missing value` (no `=`) |
| `=value`          | `PARSE_ERROR ... missing variable name` |
| `KEY="unclosed`   | `PARSE_ERROR ... missing closing quote` |
| `KEY=a` + `KEY=b` (same file) | `PARSE_ERROR ... duplicate variable name: KEY` |

Anything a `.env` file rejects surfaces as `EnvroError::Parse`; unreadable /
missing files surface as `EnvroError::File`. See [Validation](#validation) for
`EnvroError::Validation`.

## Out of scope

Features not implemented by design:

- **No multi-file layering** — composable configs are avoided; one path per call. Follows the “No-Inheritance” Flat principle ([CUE on inheritance](https://cuelang.org/docs/concept/configuration-use-case/#inheritance-based-configuration-languages), [Angular LIFT Flat](https://angular.io/guide/styleguide#flat)).
- **No compile-time macros** — config stays outside the binary so the same build can run with different env files or process env (deploy, containers, CI). Values are never baked in at `cargo build`.

## TODO

- coerce env vars to types
- encryption
- performance
  - proper parsing

---

## LICENSE

MIT License

Copyright (c) 2024-2026 Simone Sanfratello

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

