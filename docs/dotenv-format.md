# `.env` format

Small, explicit dialect. Values may contain `=`. Duplicate keys are a hard error. Only `${VAR}` is expanded (from other keys in the same file, then the process environment). Bare `$` is always literal. Use `\${VAR}` to keep the braced form without replacement. Unknown, empty, or invalid `${…}` becomes an empty string.

## Example file

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
# with feature "age" — ciphertext until load (see encryption.md)
# PG_PASS=Encrypted[AGE:b64:…]
```

## Encrypted values (`encryption` feature)

```bash
cargo add envro --features encryption
```

With that feature:

- Values may be `Encrypted[AGE:b64:…]` (age v1 binary, base64).
- Set `ENVRO_AGE_IDENTITY_FILE=~/path/to.key` in the same file (path only; stripped after load).
- Decrypt runs in `load_dotenv` after parse and before `${VAR}` expansion.
- Fields marked `secret` / `#[envro(secret)]` must use the marker in `.env`; process env / CI may inject plaintext (see [validation.md](./validation.md), [encryption.md](./encryption.md)).

```env
ENVRO_AGE_IDENTITY_FILE=~/.config/envro/my-project.key
PG_PASS=Encrypted[AGE:b64:…]
```

## Variable substitution

| Form | Behavior |
| --- | --- |
| `${NAME}` | Replaced from other keys in the same file (any order), else from the process environment |
| `${}` | Replaced with `""` |
| `\${}` / `\${NAME}` | Literal `${}` / `${NAME}` (skip replacement) |
| Unknown / invalid `${…}` | Replaced with `""` |
| `$NAME` / `$2a$…` | Always literal — bare `$` is a normal character |

`NAME` must match `[A-Za-z_][A-Za-z0-9_]*`. Expansion is order-independent: all keys are parsed first, then `${VAR}` refs are resolved across the file (and the process env) in as many passes as needed. Circular references resolve to empty strings. There is no `${NAME:-default}` syntax.

## Where substitution runs

- **`.env` files** — [`load_dotenv`](https://docs.rs/envro/latest/envro/fn.load_dotenv.html) parses, decrypts `Encrypted[…]` values when feature `encryption` is on, then expands.
- **`Config::from_env()`** — expands schema keys collected from the process environment before validate/coerce (same `${VAR}` rules).
- **`from_vars` / `validate_env`** — do **not** expand; pass already-expanded values, or call [`expand_vars`](https://docs.rs/envro/latest/envro/fn.expand_vars.html) yourself.

For local `.env` files, definition order does not matter: all keys are parsed first, then refs resolve.

## Valid rows

| Row | Parses to | Notes |
| --- | --- | --- |
| `# any text` | *(skipped)* | Full-line comment |
| *(empty line)* | *(skipped)* | Blank lines are ignored |
| `NAME=envro` | `NAME` = `envro` | Basic `KEY=value` |
| `KEY = value` | `KEY` = `value` | Spaces around `=` trimmed |
| `EMPTY=` | `EMPTY` = `""` | Empty value, no quotes |
| `EMPTY=""` | `EMPTY` = `""` | Empty quoted value |
| `QUOTED="a b"` | `QUOTED` = `a b` | Double-quoted value |
| `ESCAPED="say \"hi\""` | `ESCAPED` = `say "hi"` | `\"` escapes an inner quote |
| `DSN=host=db user=admin` | `DSN` = `host=db user=admin` | `=` allowed inside value |
| `HOST=h` / `URL=${HOST}` (any order) | `URL` = `h` | Order-independent `${VAR}` |
| `A=abc${B}` / `B=123` | `A` = `abc123` | Forward refs resolve |
| `BARE=$HOST` | `BARE` = `$HOST` | Bare `$` never expanded |
| `LIT=\${HOST}` | `LIT` = `${HOST}` | `\${…}` skips replacement |
| `X=${}` | `X` = `""` | Empty braces → empty |
| `X=\${}` | `X` = `${}` | `\${}` skips replacement |
| `X=${MISSING}` | `X` = `""` | Unknown `${VAR}` → empty |
| `X=${1}` | `X` = `""` | Invalid name → empty |
| `HASH=$2a$10$abc` | `HASH` = `$2a$10$abc` | `$` allowed as a normal char |
| `URL="pg://u:p@h/db"` | `URL` = `pg://u:p@h/db` | Any chars fine inside quotes |
| `KEY="line1`<br/>`line2"` | `KEY` = `line1\nline2` | Multi-line quoted value — newlines preserved |

## Multi-line values

Double-quoted values may span multiple physical lines. When a value opens with `"` and does not close on the same line, envro keeps reading lines (joining them with `\n`) until it finds a line ending with an unescaped `"`. Lines inside the quotes are taken **literally** — blank lines and `#` at the start of a line are part of the value, not comments. CRLF endings are normalized to `\n` inside the value.

```env
PEM="-----BEGIN PRIVATE KEY-----
MIIBVwIBADANBgkqhkiG9w0BAQEFAA...
-----END PRIVATE KEY-----"
```

Escapes inside quotes: `\"` — a literal `"`; a trailing `\"` on a line therefore does **not** close the value.

## Invalid rows

| Row | Error |
| --- | --- |
| `NAME value` | `PARSE_ERROR ... missing value` (no `=`) |
| `=value` | `PARSE_ERROR ... missing variable name` |
| `KEY="unclosed` | `PARSE_ERROR ... missing closing quote` |
| `KEY=a` + `KEY=b` (same file) | `PARSE_ERROR ... duplicate variable name: KEY` |

Anything a `.env` file rejects surfaces as `EnvroError::Parse`; unreadable / missing files surface as `EnvroError::File`. See [validation.md](./validation.md) for `EnvroError::Validation`.






