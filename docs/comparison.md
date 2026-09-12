# Comparison with similar crates

Envro validates env vars with a composable `Schema`, loads `.env` into a map or the process environment, and optionally derives a typed `Config`. Values always load at runtime.

This page compares envro with common Rust alternatives in the same space.

Stats below are approximate from [crates.io](https://crates.io) (as of 2026-09) and change over time.

## Quick pick

| Need | Prefer |
| --- | --- |
| Validate + typed `Config` from `.env` / process env (runtime values) | **envro** |
| Small API: parse to map and/or set process env with explicit override flag | **envro** |
| Widely used, drop-in successor to classic `dotenv` | [**dotenvy**](https://crates.io/crates/dotenvy) |
| Actively evolved 1.0 API, safer `$` handling, multi-file loaders | [**dotenv-ng**](https://crates.io/crates/dotenv-ng) |
| Serde structs from process env (not `.env` parsing itself) | [**envy**](https://crates.io/crates/envy) |
| Layered config (env + TOML/JSON + CLI) | **figment**, **config**, **procenv**, **envstack** |
| Encrypted secrets in `.env` (age) | **envro** (`age`) or [**dotenvage**](https://crates.io/crates/dotenvage) (full CLI / layering) |

Avoid the original [**dotenv**](https://crates.io/crates/dotenv) for new projects: it is unmaintained (last release ~2019). Prefer dotenvy or dotenv-ng.

## Feature matrix (`.env` loaders)

| | **envro** | **dotenvy** | **dotenv-ng** | **dotenv** (legacy) |
| --- | :---: | :---: | :---: | :---: |
| Parse `.env` → map (no process mutation) | yes | yes (`from_*` / iterators) | yes (`EnvLoader::load`) | limited |
| Set process environment | yes | yes | yes (`load_and_modify`, `unsafe`) | yes |
| Explicit override flag | yes (`override_existing`) | yes (`*_override` APIs) | yes (sequence / `--override`) | no (keep existing) |
| Treat empty process value as unset | yes (when `override_existing = false`) | no (empty counts as set) | configurable via sequence | no |
| Reject duplicate keys in one file | yes | last wins (typical) | last wins / layered | last wins |
| Variable substitution | `${VAR}` only (file + `Config::from_env` / `expand_vars`) | yes | optional (off by default) | yes |
| Multi-file layering | no (by design) | manual | yes | no |
| Multiline / richer quoting | double-quoted multiline + `\"` | yes | yes | yes |
| `export` prefix | no | yes | yes | yes |
| Composable validation (`Schema` / `Field`) | yes | no | no | no |
| Typed config derive | yes (`#[derive(Envro)]`, runtime values + defaults) | `dotenvy_macro` (bake values) | `macros` feature | `dotenv_codegen` |
| Encrypted secret values (age) | yes (`cargo add envro --features encryption`) | no | no | no |
| CLI runner | no | optional | optional | optional |
| Maintenance | active (this crate) | popular; last crates.io release 2023 | active (2026 fork) | unmaintained |
| Approx. downloads | ~9k | ~163M total | ~7k (new) | ~62M total |

## Detailed notes

### envro

- Load: `load_dotenv(path)` and `load_dotenv_in_env_vars(path, override_existing)`.
- Validate: composable `Schema` / `Field` rules on any map, a `.env` file, or the process environment; or `#[derive(Envro)]` typed `Config` (`from_vars` / `from_dotenv` / `from_env`).
- Defaults: `Field::default_value("…")` / `#[envro(default = "…")]` (required for `Option<T>`).
- `${VAR}`: expanded by `load_dotenv` and by `Config::from_env()` (also `expand_vars`); bare `$` is always literal. `from_vars` / `validate_env` do not expand.
- Duplicate keys in the same file are a parse error.
- With `override_existing = false`, non-empty process values win; unset **or empty** process values are filled from the file.
- Format is intentionally small: `#` comments, empty values, double-quoted (including multiline), `=` inside values, `${VAR}` substitution. No `export`, no multi-file merge.
- Derive encodes types/rules only — env **values** are never embedded at compile time.
- Feature `age` (`cargo add envro --features encryption`): per-value `Encrypted[AGE:b64:…]`, `ENVRO_AGE_IDENTITY_FILE` in `.env`, `Field::secret()` / `#[envro(secret)]` (CI plaintext via `from_env` OK). Hybrid PQ identities. See [encryption.md](./encryption.md). Not a SOPS/dotenvage replacement (no CLI, no multi-file layering).

### dotenvy

- De-facto standard fork of `dotenv`; suggested alternative in RUSTSEC guidance for the unmaintained original.
- Richer parsing (multiline, substitution, override helpers, `io::Read`).
- Default load keeps existing process variables; override variants force file values.
- Still widely depended on; crates.io last release was March 2023 (ecosystem discussion led to dotenv-ng).

### dotenv-ng

- Breaking 1.0 fork of dotenvy (Cachix / SecretSpec), August 2026.
- Dollar signs are **literal by default** (avoids mangling secrets like bcrypt hashes); substitution is opt-in.
- Clear split: load into a map vs `unsafe` process mutation at startup.
- Multi-path loaders, structured errors, optional macros and CLI.

### dotenv (legacy)

- Original crate; still downloaded via old lockfiles, but not maintained.
- Prefer dotenvy or dotenv-ng for new work.

## Adjacent crates (not direct dotenv clones)

| Crate | What it does relative to envro |
| --- | --- |
| [**envy**](https://crates.io/crates/envy) | Deserializes **already-set** env vars into Serde structs. Often paired with a dotenv loader. No built-in rule Schema like envro. |
| [**figment**](https://crates.io/crates/figment) / [**config**](https://crates.io/crates/config) | Full configuration stacks (files, env, defaults, precedence). Heavier than a `.env` parser + validator. |
| [**procenv**](https://crates.io/crates/procenv) | Derive-based typed config; can load `.env`, files, profiles, CLI. |
| [**envstack**](https://crates.io/crates/envstack) | Layered env + TOML (+ clap) with typed extract. |
| [**rust_dotenv**](https://crates.io/crates/rust_dotenv) / [**loadenv**](https://crates.io/crates/loadenv) | Smaller alternative loaders; much less adoption than dotenvy. |
| [**dotenvage**](https://crates.io/crates/dotenvage) | `.env` plus age encryption, CLI, file layering, keychain. Envro’s `encryption` feature is decrypt-on-load + helpers only; prefer dotenvage for a full secrets CLI. |
| [**load-dotenv**](https://crates.io/crates/load-dotenv) | Compile-time procedural macro to load `.env` while building (bakes values into the binary). |

## Where envro fits

Choose **envro** when you want:

1. Validation (`Schema` / `Field`) and optional typed `Config` in one crate.
2. A small, explicit load API (map vs process env) with clear override / empty-as-unset rules.
3. Duplicate-key rejection instead of silent last-wins.
4. `${VAR}` compose for `.env` **and** process env (`Config::from_env`), with bare `$` left literal.
5. One file per call (no built-in multi-file layering).
6. Runtime-only **values** (derive encodes types/rules/defaults only; no baked-in secrets).
7. Age-encrypted secret values in a single `.env` (PQ identities; which keys you encrypt is up to you — see [encryption.md](./encryption.md)).
