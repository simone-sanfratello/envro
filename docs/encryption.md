# Encryption (age)

Feature [`age`](https://age-encryption.org/) for keeping secrets in a local `.env` without plaintext passwords. **Never commit `.env` files.**

```bash
cargo add envro --features encryption
```

Secret **values** become age ciphertext. Non-secrets stay readable. Decrypt happens in memory when you load the file with a **per-project private identity**.

```env
ENVRO_AGE_IDENTITY_FILE=~/.config/envro/my-project.key
PG_HOST=localhost
PG_PASS=Encrypted[AGE:b64:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+…]
DATABASE_URI=pg://app:${PG_PASS}@${PG_HOST}/app
```

`PG_HOST` is plaintext. `PG_PASS` is ciphertext until `load_dotenv` unwraps it (field marked `secret`), then `${PG_PASS}` expands as usual.

## Threat model

**In scope**

- Attacker reads a leaked local `.env` (backup, chat paste, sync folder) — they see ciphertext, not the password (when every `secret` field is `Encrypted[…]`; see [Reject plaintext secrets](#reject-plaintext-secrets)).

**Out of scope**

- Compromised environments or machines (CI jobs, unlocked laptops) that already have secrets, a decrypt identity, or the running process.
- Relying on encryption so you can commit `.env` — **do not**. Keep `.env` out of git (`.gitignore`).

Encrypted `.env` is for **local files**. In CD, inject process env and use `Config::from_env()`.

## What changes vs plaintext

| | On disk (local `.env`) | After load (with identity) |
| --- | --- | --- |
| `PG_HOST` | `localhost` | `localhost` |
| `PG_PASS` | `Encrypted[AGE:b64:…]` | `s3cr3t` in memory only |

Same file, same `KEY=value` lines. Only the secret **values** change shape.

## Identity (one key per project)

Do **not** reuse one key across all repos. Keep each private key outside git; point at it **explicitly**.

| Piece | Where | Commit? |
| --- | --- | --- |
| Private identity | e.g. `~/.config/envro/my-project.key` | **No** |
| Public recipient | `age1pq1…` from keygen (share when encrypting) | **Yes** (safe to share) |
| `.env` (incl. ciphertext) | project root / local only | **No** — never commit `.env` |
| Identity path | `ENVRO_AGE_IDENTITY_FILE=…` **in `.env`** | N/A (`.env` stays local; prefer `~/…`) |

**Required in `.env`** whenever the file contains `Encrypted[…]` values (unless you pass an identity path in code):

```env
ENVRO_AGE_IDENTITY_FILE=~/.config/envro/my-project.key
PG_HOST=localhost
PG_PASS=Encrypted[AGE:b64:…]
```

That value is a **path**, not a secret. Prefer `~/…` so every teammate can use the same line. Envro reads it after parse, loads the identity, decrypts other values, then **drops** `ENVRO_AGE_IDENTITY_FILE` from the result map (it is not app config).

**Or in code:** pass an identity path to the load/encrypt helpers (tests, custom layouts).

Missing path + encrypted values → `EnvroError::Decrypt`. After unwrap, envro zeroizes identity material.

## Post-quantum keys

New project keys must be age **hybrid PQ** (ML-KEM-768 + X25519), not classic X25519.

| | |
| --- | --- |
| Public | `age1pq1…` (long) |
| Private | `AGE-SECRET-KEY-PQ-1…` |
| Generate | `age-keygen -pq -o ~/.config/envro/<project>.key` |

**PQ-only recipient lists.** Do not mix `age1pq1…` with classic `age1…` or SSH recipients on the same value — that drops post-quantum security; age rejects the mix.

Hardware (`age1tagpq1…` / YubiKey plugin) is optional and follows the same PQ-only rule.

## Marker format

```text
Encrypted[AGE:b64:<standard-base64>]
```

Base64 of age v1 **binary** ciphertext. Inner bytes are normal age ciphertext (one random DEK, wrapped to each recipient). Encrypt once to one or more `age1pq1…` public keys.

## From scratch (one project)

```bash
# 1. Project identity (on your machine — not in git)
#    Needs Go age 1.3+ for age-keygen -pq
mkdir -p ~/.config/envro
age-keygen -pq -o ~/.config/envro/my-project.key
chmod 600 ~/.config/envro/my-project.key
# note Public key: age1pq1…

# 2. .env — identity path + ciphertext (mark PG_PASS with #[envro(secret)] in code)
# ENVRO_AGE_IDENTITY_FILE=~/.config/envro/my-project.key
# PG_HOST=localhost
# PG_PASS=Encrypted[AGE:b64:…]
#    Encrypt to your age1pq1… (envro::encrypt_value or age/rage CLI)

# 3. Use envro with the age feature
cargo add envro --features encryption
cd ~/code/my-project && cargo run
# load_dotenv reads ENVRO_AGE_IDENTITY_FILE from .env, decrypts, expands ${VAR}
```

**Teammate:** they generate their own key, share their `age1pq1…`, you re-encrypt secrets to include them, then share the updated `.env` out of band (never via git).

**Next repo:** new key file + that repo’s local `.env` with its own `ENVRO_AGE_IDENTITY_FILE=…`.

## Load pipeline

With feature `encryption` enabled:

1. Parse `.env` → map.
2. Resolve identity: explicit API path, else `ENVRO_AGE_IDENTITY_FILE` from the map (then remove that key). Required if any value is `Encrypted[…]`.
3. Decrypt every `Encrypted[AGE:b64:…]` value.
4. Expand `${VAR}`.
5. Validate / set process env / `Config` as today.

Applies to `load_dotenv`, `load_dotenv_in_env_vars`, `load_dotenv_validated`, `Config::from_dotenv`.

`Config::from_env()` / `validate_env()` do **not** decrypt — process env is already plaintext.

Without feature `encryption`, an `Encrypted[…]` value is a **hard error** (do not leave ciphertext as a fake password).

## Docker Compose

Compose/`env_file:` passes values through as-is (no age decrypt).

**App decrypts inside the container:** mount the identity key; load `.env` with envro (`Config::from_dotenv` / `load_dotenv`). Put `ENVRO_AGE_IDENTITY_FILE` in that `.env`. Prefer not using Compose `env_file:` for encrypted secret keys (Compose would set ciphertext into the process environment).

```yaml
# docker-compose.yml (sketch)
services:
  app:
    image: my-project
    working_dir: /app
    volumes:
      - ./:/app
      - ${HOME}/.config/envro/my-project.key:/root/.config/envro/my-project.key:ro
    environment:
      RUST_LOG: info
```

**CD / prod:** inject plaintext with Compose `environment:` / CI secrets / Docker secrets and use `Config::from_env()` — see [Reject plaintext secrets](#reject-plaintext-secrets) (`secret` allows CI plaintext).

## Library helpers (`encryption` feature)

```rust
// Encrypt plaintext to recipients → "Encrypted[AGE:b64:…]"
envro::encrypt_value(plaintext, &recipients)?;

// Decrypt one value (or return unchanged if not marked)
envro::decrypt_value(value, &identities)?;
```

Prefer these over hand-wrapping `rage` output so the marker stays consistent.

## Reject plaintext secrets

Mark secrets with `Field::secret()` / `#[envro(secret)]` (see [validation.md](./validation.md)).

| Source | Value | Result |
| --- | --- | --- |
| `.env` | `Encrypted[AGE:b64:…]` + `secret` | Decrypt → load |
| `.env` | plaintext + `secret` | Error |
| `.env` | plaintext, no `secret` | OK |
| CI / process env | plaintext + `secret` | OK (`Config::from_env()`) |

## Rotate / revoke

Re-encrypt always mints a **new** DEK.

| Action | Steps |
| --- | --- |
| Add person | Add their `age1pq1…` → re-encrypt secrets → redistribute `.env` out of band |
| Remove person | Drop their pubkey → re-encrypt → **change the real passwords** (old copies of `.env` still decrypt with their old key) |
| Lost key | New `age-keygen -pq` → re-encrypt to the new public key → delete old private file |
| Stolen identity | Revoke + re-encrypt + **rotate every secret** that identity could read |

## Errors

`EnvroError::Decrypt { key, reason }` — missing identity, wrong key, corrupt blob, PQ/classic mix, or plaintext on a `secret` field.

## See also

- [dotenv-format.md](./dotenv-format.md) — dialect + encrypted value rows
- [api.md](./api.md) — `encrypt_value` / `decrypt_value` / `EnvroError::Decrypt`
- [age](https://age-encryption.org/) / [rage](https://github.com/str4d/rage) — format and CLI; apps use the `age` crate
- Example: `cd example && cargo run --bin encryption` (needs a real `Encrypted[…]` value and `~/.config/envro/example.key`)
