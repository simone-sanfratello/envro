# Comparison with similar crates

Envro loads variables from a `.env` file: either as a map (`load_dotenv`) or into the process environment (`load_dotenv_in_env_vars`).

This page compares envro with common Rust alternatives in the same space.

Stats below are approximate from [crates.io](https://crates.io) (as of 2026-09) and change over time.

## Quick pick

| Need | Prefer |
| --- | --- |
| Small API: parse to map and/or set process env with explicit override flag | **envro** |
| Widely used, drop-in successor to classic `dotenv` | [**dotenvy**](https://crates.io/crates/dotenvy) |
| Actively evolved 1.0 API, safer `$` handling, multi-file loaders | [**dotenv-ng**](https://crates.io/crates/dotenv-ng) |
| Serde structs from process env (not `.env` parsing itself) | [**envy**](https://crates.io/crates/envy) |
| Layered config (env + TOML/JSON + CLI) | **figment**, **config**, **procenv**, **envstack** |
| Encrypted secrets in `.env` | [**dotenvage**](https://crates.io/crates/dotenvage) |

Avoid the original [**dotenv**](https://crates.io/crates/dotenv) for new projects: it is unmaintained (last release ~2019). Prefer dotenvy or dotenv-ng.

## Feature matrix (`.env` loaders)

| | **envro** | **dotenvy** | **dotenv-ng** | **dotenv** (legacy) |
| --- | :---: | :---: | :---: | :---: |
| Parse `.env` → map (no process mutation) | yes | yes (`from_*` / iterators) | yes (`EnvLoader::load`) | limited |
| Set process environment | yes | yes | yes (`load_and_modify`, `unsafe`) | yes |
| Explicit override flag | yes (`override_existing`) | yes (`*_override` APIs) | yes (sequence / `--override`) | no (keep existing) |
| Treat empty process value as unset | yes (when `override_existing = false`) | no (empty counts as set) | configurable via sequence | no |
| Reject duplicate keys in one file | yes | last wins (typical) | last wins / layered | last wins |
| Variable substitution (`$VAR`) | no | yes | optional (off by default) | yes |
| Multi-file layering | no | manual | yes | no |
| Multiline / richer quoting | double quotes + `\"` | yes | yes | yes |
| `export` prefix | no | yes | yes | yes |
| Compile-time macros | no | `dotenvy_macro` | `macros` feature | `dotenv_codegen` |
| CLI runner | no | optional | optional | optional |
| Maintenance | active (this crate) | popular; last crates.io release 2023 | active (2026 fork) | unmaintained |
| Approx. downloads | small | ~161M total | ~7k (new) | ~62M total |

## Detailed notes

### envro

- Two functions: `load_dotenv(path)` and `load_dotenv_in_env_vars(path, override_existing)`.
- Duplicate keys in the same file are a parse error.
- With `override_existing = false`, non-empty process values win; unset **or empty** process values are filled from the file.
- Format is intentionally small: `#` comments, empty values, double-quoted strings, `=` inside values. No `$` expansion, no `export`, no multi-file merge.

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
| [**envy**](https://crates.io/crates/envy) | Deserializes **already-set** env vars into Serde structs. Often paired with a dotenv loader. |
| [**figment**](https://crates.io/crates/figment) / [**config**](https://crates.io/crates/config) | Full configuration stacks (files, env, defaults, precedence). Heavier than a `.env` parser. |
| [**procenv**](https://crates.io/crates/procenv) | Derive-based typed config; can load `.env`, files, profiles, CLI. |
| [**envstack**](https://crates.io/crates/envstack) | Layered env + TOML (+ clap) with typed extract. |
| [**rust_dotenv**](https://crates.io/crates/rust_dotenv) / [**loadenv**](https://crates.io/crates/loadenv) | Smaller alternative loaders; much less adoption than dotenvy. |
| [**dotenvage**](https://crates.io/crates/dotenvage) | `.env` plus age encryption for secrets. |
| [**load-dotenv**](https://crates.io/crates/load-dotenv) | Compile-time procedural macro to load `.env` while building. |

## Where envro fits

Choose **envro** when you want:

1. A small, explicit API (map vs process env).
2. Duplicate-key rejection instead of silent last-wins.
3. Empty process values treated like unset when not overriding.

Choose **dotenvy** / **dotenv-ng** when you need substitution, multiline dialect compatibility, multi-file layering, macros, or a CLI.

Choose **envy** / **figment** / **procenv** when the goal is typed application config rather than only loading a `.env` file.
