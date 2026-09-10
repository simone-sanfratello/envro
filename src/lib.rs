use std::collections::HashMap;
use std::io;
use std::{env, fs, path::Path};

mod validate;
pub use validate::{load_dotenv_validated, validate, validate_env, Field, Schema, ValidationIssue};

#[derive(Debug, thiserror::Error)]
pub enum EnvroError {
    #[error("FILE_ERROR unable to read env file {file:?}: {source:?}")]
    File {
        #[source]
        source: io::Error,
        file: String,
    },
    #[error("PARSE_ERROR line {line:?} is not valid: {reason}")]
    Parse { line: String, reason: String },
    #[error("VALIDATION_ERROR {}", validate::format_issues(errors))]
    Validation { errors: Vec<ValidationIssue> },
}

pub type EnvroVars = HashMap<String, String>;
/// Does `s` end with an unescaped `"`?
///
/// A trailing `"` counts as closing only when preceded by an even number of
/// backslashes (0, 2, ...). One `\` before the quote is `\"` (escaped);
/// two are `\\"` (escaped backslash + real closing quote).
fn line_closes_quote(s: &str) -> bool {
    if !s.ends_with('"') {
        return false;
    }
    let bytes = s.as_bytes();
    // Count consecutive backslashes immediately before the final '"'.
    let mut count = 0usize;
    if bytes.len() >= 2 {
        let mut i = bytes.len() - 2;
        loop {
            if bytes[i] == b'\\' {
                count += 1;
                if i == 0 {
                    break;
                }
                i -= 1;
            } else {
                break;
            }
        }
    }
    count.is_multiple_of(2)
}

fn is_var_name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn is_var_name_continue(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Collect `${NAME}` refs in `s`, ignoring `\${`.
fn var_refs_in(s: &str) -> Vec<String> {
    let bytes = s.as_bytes();
    let mut refs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 2 < bytes.len() && bytes[i + 1] == b'$' && bytes[i + 2] == b'{'
        {
            i += 3;
            continue;
        }
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            let name_start = i + 2;
            let mut j = name_start;
            while j < bytes.len() && bytes[j] != b'}' {
                j += 1;
            }
            if j < bytes.len() {
                let name = &s[name_start..j];
                if !name.is_empty()
                    && is_var_name_start(name.as_bytes()[0])
                    && name.as_bytes()[1..]
                        .iter()
                        .copied()
                        .all(is_var_name_continue)
                {
                    refs.push(name.to_string());
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    refs
}

/// Expand `${VAR}` using `known` file vars, then process env.
///
/// - Bare `$` is always literal
/// - `\${` → literal `${` (skip replacement)
/// - Known key → its value; missing → `""`
/// - Name must match `[A-Za-z_][A-Za-z0-9_]*`
fn substitute_vars(input: &str, known: &EnvroVars) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 2 < bytes.len() && bytes[i + 1] == b'$' && bytes[i + 2] == b'{'
        {
            out.push_str("${");
            i += 3;
            continue;
        }

        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            let name_start = i + 2;
            let mut j = name_start;
            while j < bytes.len() && bytes[j] != b'}' {
                j += 1;
            }
            if j >= bytes.len() {
                out.push_str(&input[i..]);
                break;
            }
            let name = &input[name_start..j];
            let name_ok = !name.is_empty()
                && is_var_name_start(name.as_bytes()[0])
                && name.as_bytes()[1..]
                    .iter()
                    .copied()
                    .all(is_var_name_continue);
            if name_ok {
                if let Some(val) = known.get(name) {
                    out.push_str(val);
                } else if let Ok(val) = env::var(name) {
                    out.push_str(&val);
                }
                // else: unknown → empty
            }
            // Invalid / empty `${}` → empty (same as unknown).
            // Use `\${` to keep a literal `${`.
            i = j + 1;
            continue;
        }

        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Resolve `${VAR}` refs so definition order does not matter.
///
/// Keys are expanded only once all of their in-file dependencies are resolved.
/// Cycles (and anything left after that) expand with missing in-file refs as `""`.
fn resolve_vars(raw: &EnvroVars) -> EnvroVars {
    let mut resolved = EnvroVars::with_capacity(raw.len());
    let mut pending: Vec<String> = raw.keys().cloned().collect();

    while !pending.is_empty() {
        let mut progress = false;
        let mut still = Vec::new();
        for key in pending {
            let raw_val = raw.get(&key).unwrap();
            let ready = var_refs_in(raw_val).iter().all(|r| {
                // Env-only refs are always ready; in-file refs must be resolved.
                !raw.contains_key(r) || resolved.contains_key(r)
            });
            if ready {
                resolved.insert(key, substitute_vars(raw_val, &resolved));
                progress = true;
            } else {
                still.push(key);
            }
        }
        if !progress {
            // Cycles: expand against a snapshot that omits still-pending keys
            // so refs into the cycle become empty.
            let frozen = resolved.clone();
            for key in still {
                let raw_val = raw.get(&key).unwrap();
                resolved.insert(key, substitute_vars(raw_val, &frozen));
            }
            break;
        }
        pending = still;
    }

    resolved
}

/// load .env file into process.env var
///
/// # Examples
///
/// ```
/// use std::env;
/// use envro::*;
///
/// let env_file = env::current_dir().unwrap().join(".env-sample");
/// let env_vars = load_dotenv(&env_file).unwrap();
/// ```
pub fn load_dotenv(file_name: &Path) -> Result<EnvroVars, EnvroError> {
    let file_content = match fs::read_to_string(file_name) {
        Ok(c) => c,
        Err(err) => {
            return Err(EnvroError::File {
                source: err,
                file: String::from(file_name.to_str().unwrap_or("unknown file name")),
            })
        }
    };

    let mut vars = EnvroVars::new();

    // Split on '\n' so we can advance the index across multi-line quoted values.
    let raw_lines: Vec<&str> = file_content.split('\n').collect();
    let mut i = 0;
    while i < raw_lines.len() {
        // Strip a trailing '\r' so CRLF files parse identically to LF.
        let raw = raw_lines[i].strip_suffix('\r').unwrap_or(raw_lines[i]);
        let line = raw.trim();

        if line.is_empty() {
            i += 1;
            continue;
        }
        if line.starts_with('#') {
            i += 1;
            continue;
        }

        let eq_idx = match line.find('=') {
            Some(idx) => idx,
            None => {
                return Err(EnvroError::Parse {
                    line: String::from(line),
                    reason: "missing value".to_string(),
                });
            }
        };

        let var = String::from(&line[..eq_idx]);
        let mut value = String::from(&line[eq_idx + 1..]);

        if var.is_empty() {
            return Err(EnvroError::Parse {
                line: String::from(line),
                reason: "missing variable name".to_string(),
            });
        }

        // env::set_var panics on NUL in the key.
        if var.contains('\0') {
            return Err(EnvroError::Parse {
                line: String::from(line),
                reason: "variable name contains NUL byte".to_string(),
            });
        }

        // Preserve the first line for error messages before we advance.
        let first_line = String::from(line);

        // Quoted values may span multiple lines. If the first line opens a
        // quote but does not close it, we accumulate subsequent lines with
        // '\n' between them until we find a line ending with the closing
        // quote. Single-line quoted values behave exactly as before.
        if value.starts_with('"') {
            let single_line_closed = value.len() >= 2 && line_closes_quote(&value);
            if single_line_closed {
                value = value[1..value.len() - 1].replace("\\\"", "\"");
            } else {
                let mut buf = String::from(&value[1..]);
                let mut closed = false;
                i += 1;
                while i < raw_lines.len() {
                    let next = raw_lines[i].strip_suffix('\r').unwrap_or(raw_lines[i]);
                    buf.push('\n');
                    if line_closes_quote(next) {
                        buf.push_str(&next[..next.len() - 1]);
                        closed = true;
                        break;
                    }
                    buf.push_str(next);
                    i += 1;
                }
                if !closed {
                    return Err(EnvroError::Parse {
                        line: first_line,
                        reason: "missing closing quote".to_string(),
                    });
                }
                value = buf.replace("\\\"", "\"");
            }
        }

        if value.contains('\0') {
            return Err(EnvroError::Parse {
                line: first_line,
                reason: "value contains NUL byte".to_string(),
            });
        }

        // Reject duplicate variable names in the same file.
        if vars.contains_key(&var) {
            return Err(EnvroError::Parse {
                line: first_line,
                reason: format!("duplicate variable name: {}", var),
            });
        }

        // Store raw values first; `${VAR}` is resolved after the whole file
        // is parsed so definition order does not matter.
        vars.insert(var, value);
        i += 1;
    }

    Ok(resolve_vars(&vars))
}

/// Load vars from an env file into process environment variables.
///
/// When `override_existing` is `false`, existing non-empty process values are kept;
/// unset or empty values are filled from the file. When `true`, file values always win.
///
/// # Examples
///
/// ```
/// use std::env;
/// use envro::*;
///
/// let env_file = env::current_dir().unwrap().join(".env-sample");
/// load_dotenv_in_env_vars(&env_file, false).unwrap();
/// ```
pub fn load_dotenv_in_env_vars(
    file_name: &Path,
    override_existing: bool,
) -> Result<(), EnvroError> {
    let vars = load_dotenv(file_name)?;

    for (key, value) in vars {
        if !override_existing {
            if let Ok(current) = env::var(&key) {
                if !current.is_empty() {
                    continue;
                }
            }
        }

        env::set_var(key, value);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{env, fs::File, io::Write};

    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn should_load_a_simple_dotenv_file() {
        let file_name = env::temp_dir().join(".env-simple");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VAR=value").unwrap();
        env::remove_var("VAR");

        load_dotenv_in_env_vars(file_name.as_path(), false).unwrap();

        assert_eq!(env::var("VAR"), Ok("value".to_string()));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    #[serial]
    fn should_handle_error_on_non_existing_dotenv_file() {
        let r = load_dotenv(Path::new("none"));
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(
                r#"FILE_ERROR unable to read env file "none": Os { code: 2, kind: NotFound, message: "No such file or directory" }"#
            )
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[serial]
    fn should_handle_error_on_non_existing_dotenv_file_on_win() {
        let r = load_dotenv(Path::new("none"));
        let err = r.unwrap_err();

        assert!(err.to_string().starts_with(
            r#"FILE_ERROR unable to read env file "none": Os { code: 2, kind: NotFound"#
        ));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    #[serial]
    fn should_handle_error_on_non_existing_dotenv_file_name_empty() {
        let r = load_dotenv(Path::new(""));
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(
                r#"FILE_ERROR unable to read env file "": Os { code: 2, kind: NotFound, message: "No such file or directory" }"#
            )
        );
    }

    #[test]
    #[serial]
    fn should_handle_error_on_invalid_dotenv_line() {
        let file_name = env::temp_dir().join(".env-invalid-line");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VAR value").unwrap();

        let r = load_dotenv(file_name.as_path());
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(r#"PARSE_ERROR line "VAR value" is not valid: missing value"#)
        );
    }

    #[test]
    #[serial]
    fn should_handle_error_on_invalid_dotenv_var() {
        let file_name = env::temp_dir().join(".env-invalid-var");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"=value").unwrap();

        let r = load_dotenv(file_name.as_path());
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(r#"PARSE_ERROR line "=value" is not valid: missing variable name"#)
        );
    }

    #[test]
    #[serial]
    fn should_handle_empty_values() {
        let file_name = env::temp_dir().join(".env-empty-value");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VAR=\nVAR2=\"\"\nVAR3=value").unwrap();
        env::remove_var("VAR");
        env::remove_var("VAR2");
        env::remove_var("VAR3");

        load_dotenv_in_env_vars(file_name.as_path(), false).unwrap();

        assert_eq!(env::var("VAR"), Ok("".to_string()));
        assert_eq!(env::var("VAR2"), Ok("".to_string()));
        assert_eq!(env::var("VAR3"), Ok("value".to_string()));
    }

    #[test]
    #[serial]
    fn should_handle_empty_lines() {
        let file_name = env::temp_dir().join(".env-empty-lines");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"\nVAR=1\nVAR1=asd").unwrap();
        env::remove_var("VAR");
        env::remove_var("VAR1");

        load_dotenv_in_env_vars(file_name.as_path(), false).unwrap();

        assert_eq!(env::var("VAR"), Ok("1".to_string()));
        assert_eq!(env::var("VAR1"), Ok("asd".to_string()));
    }

    #[test]
    #[serial]
    fn should_handle_comment_lines() {
        let file_name = env::temp_dir().join(".env-empty-lines");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"\nVAR=1\n#VAR1=asd").unwrap();
        env::remove_var("VAR");
        env::remove_var("VAR1");

        load_dotenv_in_env_vars(file_name.as_path(), false).unwrap();

        assert_eq!(env::var("VAR"), Ok("1".to_string()));
        assert_eq!(env::var("VAR1"), Err(env::VarError::NotPresent));
    }

    #[test]
    #[serial]
    fn should_handle_quoted_values() {
        let file_name = env::temp_dir().join(".env-quoted");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"\nVAR1=\"1\"\nVAR2=\"Lorem ipsum \"ciao!\" \"")
            .unwrap();
        env::remove_var("VAR1");
        env::remove_var("VAR2");

        load_dotenv_in_env_vars(file_name.as_path(), false).unwrap();

        assert_eq!(env::var("VAR1"), Ok("1".to_string()));
        assert_eq!(env::var("VAR2"), Ok("Lorem ipsum \"ciao!\" ".to_string()));
    }

    #[test]
    #[serial]
    fn should_handle_quoted_values_containg_equals() {
        let file_name = env::temp_dir().join(".env-quoted-equals");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(
            b"\nVAR1=\"1\"\nVAR2=\"host=localhost user=admin password=secret dbname=mydb\"",
        )
        .unwrap();
        env::remove_var("VAR1");
        env::remove_var("VAR2");

        load_dotenv_in_env_vars(file_name.as_path(), false).unwrap();

        assert_eq!(env::var("VAR1"), Ok("1".to_string()));
        assert_eq!(
            env::var("VAR2"),
            Ok("host=localhost user=admin password=secret dbname=mydb".to_string())
        );
    }

    #[test]
    #[serial]
    fn should_handle_invalid_quoted_values() {
        let file_name = env::temp_dir().join(".env-invalid-quoted");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(
            b"\nVAR1=\"1\"\nVAR2=\"host=localhost user=admin password=secret dbname=mydb",
        )
        .unwrap();
        env::remove_var("VAR1");
        env::remove_var("VAR2");

        let r = load_dotenv(file_name.as_path());
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(
                r#"PARSE_ERROR line "VAR2=\"host=localhost user=admin password=secret dbname=mydb" is not valid: missing closing quote"#
            )
        );
    }

    #[test]
    #[serial]
    fn should_not_ovveride_env_vars() {
        env::remove_var("VAR1");
        env::remove_var("VAR2");
        env::remove_var("VAR3");

        let file_name = env::temp_dir().join(".env-not-override");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"\nVAR1=\"value1\"\nVAR2=2\nVAR3=3")
            .unwrap();

        env::set_var("VAR1", "current-value");

        load_dotenv_in_env_vars(file_name.as_path(), false).unwrap();

        assert_eq!(env::var("VAR1"), Ok("current-value".to_string()));
        assert_eq!(env::var("VAR2"), Ok("2".to_string()));
        assert_eq!(env::var("VAR3"), Ok("3".to_string()));
    }

    #[test]
    #[serial]
    fn should_detect_duplicate_variable_names() {
        let file_name = env::temp_dir().join(".env-duplicate");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VAR1=value1\nVAR2=value2\nVAR1=value3")
            .unwrap();

        let r = load_dotenv(file_name.as_path());
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(
                r#"PARSE_ERROR line "VAR1=value3" is not valid: duplicate variable name: VAR1"#
            )
        );
    }

    #[test]
    #[serial]
    fn should_override_env_vars_when_enabled() {
        env::remove_var("VAR1");
        env::remove_var("VAR2");

        let file_name = env::temp_dir().join(".env-override");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VAR1=from-file\nVAR2=2").unwrap();

        env::set_var("VAR1", "current-value");

        load_dotenv_in_env_vars(file_name.as_path(), true).unwrap();

        assert_eq!(env::var("VAR1"), Ok("from-file".to_string()));
        assert_eq!(env::var("VAR2"), Ok("2".to_string()));
    }

    #[test]
    #[serial]
    fn should_substitute_braced_vars_from_file_and_env() {
        env::remove_var("ENVRO_TEST_FROM_ENV");
        env::set_var("ENVRO_TEST_FROM_ENV", "from-env");

        let file_name = env::temp_dir().join(".env-dollar-sub");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(
            b"HOST=example.com\n\
BARE=$HOST\n\
BRACE=${HOST}\n\
MIXED=prefix-${HOST}-suffix\n\
QUOTED=\"url://${HOST}/path\"\n\
FROM_ENV=${ENVRO_TEST_FROM_ENV}\n\
ESCAPED=\\${HOST}\n\
UNKNOWN=${DOES_NOT_EXIST}",
        )
        .unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        // Bare $HOST is never expanded — `$` is a normal character.
        assert_eq!(vars.get("BARE").map(String::as_str), Some("$HOST"));
        assert_eq!(vars.get("BRACE").map(String::as_str), Some("example.com"));
        assert_eq!(
            vars.get("MIXED").map(String::as_str),
            Some("prefix-example.com-suffix")
        );
        assert_eq!(
            vars.get("QUOTED").map(String::as_str),
            Some("url://example.com/path")
        );
        assert_eq!(vars.get("FROM_ENV").map(String::as_str), Some("from-env"));
        // `\${HOST}` skips replacement → literal `${HOST}`
        assert_eq!(vars.get("ESCAPED").map(String::as_str), Some("${HOST}"));
        // Unknown `${VAR}` → empty string
        assert_eq!(vars.get("UNKNOWN").map(String::as_str), Some(""));

        env::remove_var("ENVRO_TEST_FROM_ENV");
    }

    #[test]
    #[serial]
    fn should_treat_empty_braces_as_empty_unless_escaped() {
        let file_name = env::temp_dir().join(".env-dollar-empty-braces");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(
            b"EMPTY=${}\n\
AROUND=pre${}post\n\
ESCAPED=\\${}\n\
ESCAPED_AROUND=pre\\${}post\n\
INVALID=${123}\n\
INVALID_NAME=${bad-name}",
        )
        .unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        // `${}` → empty
        assert_eq!(vars.get("EMPTY").map(String::as_str), Some(""));
        assert_eq!(vars.get("AROUND").map(String::as_str), Some("prepost"));
        // `\${}` → no replace
        assert_eq!(vars.get("ESCAPED").map(String::as_str), Some("${}"));
        assert_eq!(
            vars.get("ESCAPED_AROUND").map(String::as_str),
            Some("pre${}post")
        );
        // invalid braces also → empty
        assert_eq!(vars.get("INVALID").map(String::as_str), Some(""));
        assert_eq!(vars.get("INVALID_NAME").map(String::as_str), Some(""));
    }

    #[test]
    #[serial]
    fn should_keep_unclosed_brace_ref_literal() {
        let file_name = env::temp_dir().join(".env-dollar-unclosed");
        let mut file = File::create(&file_name).unwrap();
        // No closing `}` — the `${HOST` tail is kept literal.
        file.write_all(b"A=pre${HOST\nB=ok").unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(vars.get("A").map(String::as_str), Some("pre${HOST"));
        assert_eq!(vars.get("B").map(String::as_str), Some("ok"));
    }

    #[test]
    #[serial]
    fn should_keep_bare_dollar_signs_literal() {
        let file_name = env::temp_dir().join(".env-dollar-literal");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(
            b"HOST=example.com\n\
PASSWORD_HASH=$2a$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy\n\
REF=$HOST\n\
QUOTED=\"cost=$2a$10$abc\"\n\
DOLLAR_ONLY=$\n\
DIGIT=$1",
        )
        .unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(
            vars.get("PASSWORD_HASH").map(String::as_str),
            Some("$2a$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy")
        );
        assert_eq!(vars.get("REF").map(String::as_str), Some("$HOST"));
        assert_eq!(
            vars.get("QUOTED").map(String::as_str),
            Some("cost=$2a$10$abc")
        );
        assert_eq!(vars.get("DOLLAR_ONLY").map(String::as_str), Some("$"));
        assert_eq!(vars.get("DIGIT").map(String::as_str), Some("$1"));
    }

    #[test]
    #[serial]
    fn should_resolve_vars_regardless_of_order() {
        let file_name = env::temp_dir().join(".env-dollar-order");
        let mut file = File::create(&file_name).unwrap();
        // A references B before B is defined — must still expand.
        file.write_all(b"A=abc${B}\nB=123").unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(vars.get("A").map(String::as_str), Some("abc123"));
        assert_eq!(vars.get("B").map(String::as_str), Some("123"));
    }

    #[test]
    #[serial]
    fn should_resolve_forward_and_chained_references() {
        let file_name = env::temp_dir().join(".env-dollar-chain");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"A=${B}${C}\nB=${C}\nC=x").unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(vars.get("A").map(String::as_str), Some("xx"));
        assert_eq!(vars.get("B").map(String::as_str), Some("x"));
        assert_eq!(vars.get("C").map(String::as_str), Some("x"));
    }

    #[test]
    #[serial]
    fn should_resolve_cycles_to_empty() {
        let file_name = env::temp_dir().join(".env-dollar-cycle");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"A=${B}\nB=${A}").unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(vars.get("A").map(String::as_str), Some(""));
        assert_eq!(vars.get("B").map(String::as_str), Some(""));
    }

    #[test]
    #[serial]
    fn should_reject_lone_opening_quote_without_panic() {
        let file_name = env::temp_dir().join(".env-lone-quote");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VAR=\"").unwrap();

        let err = load_dotenv(file_name.as_path()).unwrap_err();
        assert!(
            err.to_string().contains("missing closing quote"),
            "got: {err}"
        );
    }

    #[test]
    #[serial]
    fn should_reject_nul_in_value() {
        let file_name = env::temp_dir().join(".env-nul-value");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VAR=a\0b").unwrap();

        let err = load_dotenv(file_name.as_path()).unwrap_err();
        assert!(
            err.to_string().contains("value contains NUL byte"),
            "got: {err}"
        );
    }

    #[test]
    #[serial]
    fn should_reject_nul_in_key() {
        let file_name = env::temp_dir().join(".env-nul-key");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"VA\0R=ok").unwrap();

        let err = load_dotenv(file_name.as_path()).unwrap_err();
        assert!(
            err.to_string().contains("variable name contains NUL byte"),
            "got: {err}"
        );
    }
    #[test]
    #[serial]
    fn should_load_multiline_quoted_value() {
        let file_name = env::temp_dir().join(".env-multiline");
        let mut file = File::create(&file_name).unwrap();
        // KEY spans three physical lines; newlines are preserved in the value.
        file.write_all(
            b"BEFORE=before\n\
KEY=\"line1\n\
line2\n\
line3\"\n\
AFTER=after",
        )
        .unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(vars.get("BEFORE").map(String::as_str), Some("before"));
        assert_eq!(
            vars.get("KEY").map(String::as_str),
            Some("line1\nline2\nline3")
        );
        assert_eq!(vars.get("AFTER").map(String::as_str), Some("after"));
    }

    #[test]
    #[serial]
    fn should_preserve_blank_and_special_lines_inside_multiline_quote() {
        let file_name = env::temp_dir().join(".env-multiline-mixed");
        let mut file = File::create(&file_name).unwrap();
        // Blank lines and lines starting with `#` inside the quotes are part
        // of the value, not comments/skips.
        file.write_all(
            b"BLOB=\"line1\n\
\n\
# not a comment\n\
line=with=equals\n\
line4\"",
        )
        .unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(
            vars.get("BLOB").map(String::as_str),
            Some("line1\n\n# not a comment\nline=with=equals\nline4")
        );
    }

    #[test]
    #[serial]
    fn should_support_escaped_quote_inside_multiline_value() {
        let file_name = env::temp_dir().join(".env-multiline-escape");
        let mut file = File::create(&file_name).unwrap();
        // \" escapes an inner quote, even across lines.
        file.write_all(
            b"MSG=\"first\n\
second \\\"hello\\\"\n\
third\"",
        )
        .unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(
            vars.get("MSG").map(String::as_str),
            Some("first\nsecond \"hello\"\nthird")
        );
    }

    #[test]
    #[serial]
    fn should_reject_unclosed_multiline_quote() {
        let file_name = env::temp_dir().join(".env-multiline-unclosed");
        let mut file = File::create(&file_name).unwrap();
        // Opens on VAR2, never closes.
        file.write_all(b"VAR1=1\nVAR2=\"start\nmiddle\nno close")
            .unwrap();

        let r = load_dotenv(file_name.as_path());
        let err = r.unwrap_err();

        assert_eq!(
            err.to_string(),
            String::from(r#"PARSE_ERROR line "VAR2=\"start" is not valid: missing closing quote"#)
        );
    }

    #[test]
    #[serial]
    fn should_handle_crlf_line_endings_including_multiline() {
        let file_name = env::temp_dir().join(".env-crlf-multiline");
        let mut file = File::create(&file_name).unwrap();
        file.write_all(b"A=1\r\nB=\"one\r\ntwo\"\r\nC=3\r\n")
            .unwrap();

        let vars = load_dotenv(file_name.as_path()).unwrap();

        assert_eq!(vars.get("A").map(String::as_str), Some("1"));
        assert_eq!(vars.get("B").map(String::as_str), Some("one\ntwo"));
        assert_eq!(vars.get("C").map(String::as_str), Some("3"));
    }
}
