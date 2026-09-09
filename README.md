# Envro

A crate to load environment variables from a `.env` file into the process environment.

### Getting started

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

## API

### Types

- `EnvroVars` — alias for `HashMap<String, String>`
- `EnvroError` — errors from reading or parsing a `.env` file
  - `EnvroError::File` — cannot read the file
  - `EnvroError::Parse` — invalid line (missing `=`, empty name, unclosed quote, duplicate key, …)

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

## `.env` format

```env
# comments are ignored
DB_CONNECTION_STRING=pg://user:pass@db/mydb
DB_POOL_SIZE=32
EMPTY=
QUOTED="value with spaces"
ESCAPED="say \"hello\""
WITH_EQUALS=host=localhost user=admin
```

Supported:

- empty lines and `#` comments
- empty values (`KEY=` or `KEY=""`)
- double-quoted values, with `\"` escapes
- `=` inside values (unquoted or quoted)

Rejected:

- missing `=` / missing value
- empty variable name (`=value`)
- unclosed double quote
- duplicate variable names in the same file

## Test

```bash
cargo test
```

## TODO

- values validation
- github action publish
  - publish crate - see https://github.com/googleapis/release-please
    - run cz bump on CI, create release commit, create github release, cargo publish
      - handle pre release (-dev, -beta ...)
- coerce env vars to types

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
