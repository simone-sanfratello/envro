# Validation

Envro validates env vars with a composable rule set. Validation is **opt-in** and **decoupled from loading**: the same `Schema` works on any `HashMap<String, String>`, a `.env` file, or the live process environment. Every failing rule is collected and returned at once.

## At a glance

- Build a `Schema` of `Field` specs, one per env var.
- Every `Field` starts as `Field::required()` or `Field::default_value(...)`, then chains rules.
- Prefer `#[derive(Envro)]` when you also want typed fields — see the [README](../README.md) and [api.md](./api.md). Hand-written `Schema` stays useful for maps, tests, and validating without a struct.

```rust
use envro::*;

let schema = Schema::new()
    .field("APP_NAME", Field::required().min_len(1).max_len(64))
    .field("APP_PORT", Field::required().port())
    .field(
        "DATABASE_URL",
        Field::required().min_len(1).starts_with("postgres://"),
    )
    .field("DB_POOL_SIZE", Field::required().positive_integer())
    .field(
        "LOG_LEVEL",
        Field::required().one_of(&["debug", "info", "warn", "error"]),
    )
    .field("FEATURE_METRICS", Field::required().boolean());

validate_env(&schema)?;
```

## Semantics

| Situation | Behavior |
| --- | --- |
| Key missing from the map | `required` fails; `default_value(...)` validates `default` |
| Key present with `""` | Same as missing (empty is treated as absent) |
| Key present with a value | Every rule on the field is checked; all failures are collected |
| Key in the map but not in the schema | Ignored |
| Multiple failures on one field | All collected |
| Multiple failing fields | All collected in one `EnvroError::Validation` |
| Length rules (`min_len`, `max_len`, `exact_len`) | Count Unicode scalars, not bytes |

## Presence

Every field starts here. It is the only rule that is **not** chained.

```rust
Field::required()                    // must be present and non-empty
Field::default_value("info").min_len(3)   // missing/empty → use "info", then rules
```

```env
# required  -> fails
APP_NAME=

# required  -> passes
APP_NAME=envro

# default_value("info")  -> passes (uses default "info")
```

With `#[derive(Envro)]`:

```rust
#[envro(default = "info", one_of("debug", "info", "warn", "error"))]
log_level: String,

#[envro(integer, default = "0")]
offset: Option<i64>,  // Option<T> requires default =
```

## Rule reference

Each entry shows a schema snippet and one `.env` value that passes and one that fails.

### Length / string shape

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

### Numbers

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

- `float()` — parses as finite `f64` (`nan` / `inf` rejected)
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

### Boolean

- `boolean()` — one of `true` / `false` / `1` / `0` / `yes` / `no` (case-insensitive)
  ```rust
  Field::required().boolean()
  ```
  ```env
  FEATURE_X=true    # ok
  FEATURE_X=YES     # ok (case-insensitive)
  FEATURE_X=on      # fail: boolean (not accepted)
  ```

### Formats (best-effort, std-only)

- `email()` — `local@domain`, no whitespace, both sides non-empty. Practical, not RFC 5322.
  ```rust
  Field::default_value("").email()
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

### Lists

`list(delim, item)` splits the value on `delim`, trims each part, and applies `item`'s rules to every element. Failing elements report the key as `KEY[i]` where `i` is the 0-based position. `min_items(n)` and `max_items(n)` chain after `list(..)` to bound the number of elements.

- required items, non-empty parts required:
  ```rust
  Field::required().list(',', Field::required().min_len(1))
  ```
  ```env
  TAGS=alpha,beta,gamma   # ok
  TAGS=alpha,,gamma       # fail: TAGS[1] required
  ```

- optional items, empty parts use the item default:
  ```rust
  Field::required().list(',', Field::default_value("xx").min_len(2))
  ```
  ```env
  T=aa,,cc     # ok (empty middle → "xx")
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

## Inspecting errors

`EnvroError::Validation` carries the full list of `ValidationIssue`s, and `Display` joins them under a `VALIDATION_ERROR ...` prefix:

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

Common `#[envro(...)]` attrs mirror these `Field` rules: flags such as `port`, `boolean`, `integer`, `positive_integer`, `email`, … and keyed forms `min_len = n`, `max_len = n`, `starts_with = "..."`, `one_of("a", "b")`, `int_range(1, 100)`, etc.
