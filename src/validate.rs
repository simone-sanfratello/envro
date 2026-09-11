//! Composable value validation.
//!
//! Validation is **decoupled** from `.env` loading: it works on any
//! `HashMap<String, String>` ([`EnvroVars`]) and on the live process
//! environment. Use whichever entry point matches where your env vars come
//! from:
//!
//! - [`validate`] — validate any already-built `EnvroVars` map (from a `.env`
//!   file, a CLI, a config service, tests, …).
//! - [`validate_env`] — validate the current process environment directly.
//! - [`load_dotenv_validated`] — one-shot helper: load a `.env` file and
//!   validate the resulting map.
//!
//! # Examples
//!
//! Validate an arbitrary map:
//!
//! ```
//! use envro::*;
//!
//! let mut vars = EnvroVars::new();
//! vars.insert("APP_NAME".to_string(), "envro".to_string());
//! vars.insert("PORT".to_string(), "8080".to_string());
//! vars.insert("LOG_LEVEL".to_string(), "info".to_string());
//!
//! let schema = Schema::new()
//!     .field("APP_NAME", Field::required().min_len(3).max_len(64))
//!     .field("PORT", Field::required().port())
//!     .field("ADMIN_EMAIL", Field::optional().email())
//!     .field("LOG_LEVEL", Field::required().one_of(&["debug", "info", "warn", "error"]));
//!
//! validate(&vars, &schema).unwrap();
//! ```

use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::str::FromStr;

use crate::{load_dotenv, EnvroError, EnvroVars};

/// A single validation failure for one key (or list element).
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Env var name; list elements use `KEY[i]`.
    pub key: String,
    /// Rule name that failed (e.g. `required`, `min_len`, `port`).
    pub rule: &'static str,
    /// Human-readable reason.
    pub reason: String,
}

impl std::fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]: {}", self.key, self.rule, self.reason)
    }
}

/// Format a list of issues for [`EnvroError::Validation`]'s `Display`.
pub(crate) fn format_issues(issues: &[ValidationIssue]) -> String {
    issues
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

#[derive(Debug, Clone, Copy)]
enum Presence {
    Required,
    Optional,
}

#[derive(Debug)]
enum Rule {
    // string shape
    MinLen(usize),
    MaxLen(usize),
    ExactLen(usize),
    Alpha,
    Alphanumeric,
    Digits,
    Ascii,
    Lowercase,
    Uppercase,
    StartsWith(String),
    EndsWith(String),
    Contains(String),
    OneOf(Vec<String>),
    NotOneOf(Vec<String>),
    // numbers
    Integer,
    PositiveInteger,
    NonNegativeInteger,
    Float,
    PositiveFloat,
    NonNegativeFloat,
    IntRange(i64, i64),
    FloatRange(f64, f64),
    Port,
    // boolean
    Boolean,
    // formats
    Email,
    Url,
    Uuid,
    Ipv4,
    Ip,
    Hex,
    // list
    List {
        delim: char,
        item: Box<Field>,
        min_items: Option<usize>,
        max_items: Option<usize>,
    },
}

/// A composable field spec.
///
/// Start with [`Field::required`] or [`Field::optional`], then chain rules.
/// See the module docs for an example.
#[derive(Debug)]
pub struct Field {
    presence: Presence,
    rules: Vec<Rule>,
}

impl Field {
    /// Field must be present in the map and have a non-empty value.
    pub fn required() -> Self {
        Self {
            presence: Presence::Required,
            rules: Vec::new(),
        }
    }

    /// Field may be missing or empty; other rules are skipped when so.
    pub fn optional() -> Self {
        Self {
            presence: Presence::Optional,
            rules: Vec::new(),
        }
    }

    // --- string shape ---

    /// Minimum length in characters (not bytes).
    pub fn min_len(mut self, n: usize) -> Self {
        self.rules.push(Rule::MinLen(n));
        self
    }
    /// Maximum length in characters (not bytes).
    pub fn max_len(mut self, n: usize) -> Self {
        self.rules.push(Rule::MaxLen(n));
        self
    }
    /// Exact length in characters (not bytes).
    pub fn exact_len(mut self, n: usize) -> Self {
        self.rules.push(Rule::ExactLen(n));
        self
    }
    /// Only alphabetic characters.
    pub fn alpha(mut self) -> Self {
        self.rules.push(Rule::Alpha);
        self
    }
    /// Only alphanumeric characters.
    pub fn alphanumeric(mut self) -> Self {
        self.rules.push(Rule::Alphanumeric);
        self
    }
    /// Only ASCII digits `0-9`.
    pub fn digits(mut self) -> Self {
        self.rules.push(Rule::Digits);
        self
    }
    /// Only ASCII characters.
    pub fn ascii(mut self) -> Self {
        self.rules.push(Rule::Ascii);
        self
    }
    /// No uppercase characters.
    pub fn lowercase(mut self) -> Self {
        self.rules.push(Rule::Lowercase);
        self
    }
    /// No lowercase characters.
    pub fn uppercase(mut self) -> Self {
        self.rules.push(Rule::Uppercase);
        self
    }
    /// Must start with the given substring.
    pub fn starts_with(mut self, s: impl Into<String>) -> Self {
        self.rules.push(Rule::StartsWith(s.into()));
        self
    }
    /// Must end with the given substring.
    pub fn ends_with(mut self, s: impl Into<String>) -> Self {
        self.rules.push(Rule::EndsWith(s.into()));
        self
    }
    /// Must contain the given substring.
    pub fn contains(mut self, s: impl Into<String>) -> Self {
        self.rules.push(Rule::Contains(s.into()));
        self
    }
    /// Must equal one of the given options.
    pub fn one_of<S: AsRef<str>>(mut self, options: &[S]) -> Self {
        self.rules.push(Rule::OneOf(
            options.iter().map(|s| s.as_ref().to_string()).collect(),
        ));
        self
    }
    /// Must not equal any of the given options.
    pub fn not_one_of<S: AsRef<str>>(mut self, options: &[S]) -> Self {
        self.rules.push(Rule::NotOneOf(
            options.iter().map(|s| s.as_ref().to_string()).collect(),
        ));
        self
    }

    // --- numbers ---

    /// Parses as `i64`.
    pub fn integer(mut self) -> Self {
        self.rules.push(Rule::Integer);
        self
    }
    /// Parses as `i64` and is `> 0`.
    pub fn positive_integer(mut self) -> Self {
        self.rules.push(Rule::PositiveInteger);
        self
    }
    /// Parses as `i64` and is `>= 0`.
    pub fn non_negative_integer(mut self) -> Self {
        self.rules.push(Rule::NonNegativeInteger);
        self
    }
    /// Parses as `f64`.
    pub fn float(mut self) -> Self {
        self.rules.push(Rule::Float);
        self
    }
    /// Parses as `f64` and is `> 0.0`.
    pub fn positive_float(mut self) -> Self {
        self.rules.push(Rule::PositiveFloat);
        self
    }
    /// Parses as `f64` and is `>= 0.0`.
    pub fn non_negative_float(mut self) -> Self {
        self.rules.push(Rule::NonNegativeFloat);
        self
    }
    /// Parses as `i64` and is within `[min, max]` (inclusive).
    pub fn int_range(mut self, min: i64, max: i64) -> Self {
        self.rules.push(Rule::IntRange(min, max));
        self
    }
    /// Parses as `f64` and is within `[min, max]` (inclusive).
    pub fn float_range(mut self, min: f64, max: f64) -> Self {
        self.rules.push(Rule::FloatRange(min, max));
        self
    }
    /// TCP/UDP port: integer in `1..=65535`.
    pub fn port(mut self) -> Self {
        self.rules.push(Rule::Port);
        self
    }

    // --- boolean ---

    /// One of `true`/`false`/`1`/`0`/`yes`/`no` (case-insensitive).
    pub fn boolean(mut self) -> Self {
        self.rules.push(Rule::Boolean);
        self
    }

    // --- formats (std-only, best-effort) ---

    /// `local@domain`, no whitespace, both sides non-empty. Best-effort, not RFC 5322.
    pub fn email(mut self) -> Self {
        self.rules.push(Rule::Email);
        self
    }
    /// Must start with `http://` or `https://` and have a non-empty, whitespace-free remainder.
    pub fn url(mut self) -> Self {
        self.rules.push(Rule::Url);
        self
    }
    /// Canonical 8-4-4-4-12 hex form.
    pub fn uuid(mut self) -> Self {
        self.rules.push(Rule::Uuid);
        self
    }
    /// Parses via `std::net::Ipv4Addr`.
    pub fn ipv4(mut self) -> Self {
        self.rules.push(Rule::Ipv4);
        self
    }
    /// Parses via `std::net::IpAddr` (v4 or v6).
    pub fn ip(mut self) -> Self {
        self.rules.push(Rule::Ip);
        self
    }
    /// Hex string, optional `0x`/`0X` prefix; remainder must be non-empty hex digits.
    pub fn hex(mut self) -> Self {
        self.rules.push(Rule::Hex);
        self
    }

    // --- list ---

    /// Split the value on `delim` (trimming each part) and apply `item`'s rules to
    /// each element. Element failures are reported with key `KEY[i]`.
    ///
    /// When `item` is [`Field::optional`], empty parts skip the item rules; when
    /// [`Field::required`], empty parts fail the `required` rule.
    pub fn list(mut self, delim: char, item: Field) -> Self {
        self.rules.push(Rule::List {
            delim,
            item: Box::new(item),
            min_items: None,
            max_items: None,
        });
        self
    }
    /// Minimum number of items in the most recently added [`Field::list`].
    ///
    /// Has no effect if the most recent rule is not a list.
    pub fn min_items(mut self, n: usize) -> Self {
        if let Some(Rule::List { min_items, .. }) = self.rules.last_mut() {
            *min_items = Some(n);
        }
        self
    }
    /// Maximum number of items in the most recently added [`Field::list`].
    ///
    /// Has no effect if the most recent rule is not a list.
    pub fn max_items(mut self, n: usize) -> Self {
        if let Some(Rule::List { max_items, .. }) = self.rules.last_mut() {
            *max_items = Some(n);
        }
        self
    }
}

/// A collection of `(key, Field)` specs.
#[derive(Debug, Default)]
pub struct Schema {
    fields: Vec<(String, Field)>,
}

impl Schema {
    /// Create an empty schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a field spec for `name`.
    pub fn field(mut self, name: impl Into<String>, field: Field) -> Self {
        self.fields.push((name.into(), field));
        self
    }
}

/// Validate an arbitrary `EnvroVars` map against a schema.
///
/// This function is **completely decoupled from any I/O**: `vars` can come
/// from [`load_dotenv`], a CLI, a config service, test fixtures, or anywhere
/// else that produces a `HashMap<String, String>`.
///
/// Collects every failing rule across every field before returning.
/// Unknown keys in `vars` (not in the schema) are ignored.
///
/// See [`validate_env`] to validate the live process environment directly.
pub fn validate(vars: &EnvroVars, schema: &Schema) -> Result<(), EnvroError> {
    let mut issues = Vec::new();
    for (key, field) in &schema.fields {
        validate_field(key, field, vars.get(key).map(String::as_str), &mut issues);
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(EnvroError::Validation { errors: issues })
    }
}

/// Load a `.env` file and validate it in one step.
pub fn load_dotenv_validated(path: &Path, schema: &Schema) -> Result<EnvroVars, EnvroError> {
    let vars = load_dotenv(path)?;
    validate(&vars, schema)?;
    Ok(vars)
}

/// Validate the **process environment** against a schema.
///
/// This is completely decoupled from `.env` file loading: for every key in the
/// schema, the value is read from `std::env::var(key)`. Missing keys or read
/// errors are treated as "not present" (same semantics as [`validate`] on a
/// `HashMap` where the key is absent).
///
/// Use this when your env vars come from anywhere other than a `.env` file
/// (shell exports, systemd unit, container runtime, `env::set_var`, tests, …).
pub fn validate_env(schema: &Schema) -> Result<(), EnvroError> {
    let mut issues = Vec::new();
    for (key, field) in &schema.fields {
        let value = std::env::var(key).ok();
        validate_field(key, field, value.as_deref(), &mut issues);
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(EnvroError::Validation { errors: issues })
    }
}

fn validate_field(key: &str, field: &Field, value: Option<&str>, out: &mut Vec<ValidationIssue>) {
    let present = matches!(value, Some(v) if !v.is_empty());
    if !present {
        if matches!(field.presence, Presence::Required) {
            out.push(ValidationIssue {
                key: key.to_string(),
                rule: "required",
                reason: "value is missing or empty".to_string(),
            });
        }
        return;
    }
    let value = value.unwrap();
    for rule in &field.rules {
        check_rule(key, value, rule, out);
    }
}

fn check_rule(key: &str, value: &str, rule: &Rule, out: &mut Vec<ValidationIssue>) {
    let issue = |name: &'static str, reason: String| ValidationIssue {
        key: key.to_string(),
        rule: name,
        reason,
    };
    match rule {
        Rule::MinLen(n) => {
            let l = value.chars().count();
            if l < *n {
                out.push(issue("min_len", format!("length {} < {}", l, n)));
            }
        }
        Rule::MaxLen(n) => {
            let l = value.chars().count();
            if l > *n {
                out.push(issue("max_len", format!("length {} > {}", l, n)));
            }
        }
        Rule::ExactLen(n) => {
            let l = value.chars().count();
            if l != *n {
                out.push(issue("exact_len", format!("length {} != {}", l, n)));
            }
        }
        Rule::Alpha => {
            if !value.chars().all(|c| c.is_alphabetic()) {
                out.push(issue("alpha", "non-alphabetic character".to_string()));
            }
        }
        Rule::Alphanumeric => {
            if !value.chars().all(|c| c.is_alphanumeric()) {
                out.push(issue(
                    "alphanumeric",
                    "non-alphanumeric character".to_string(),
                ));
            }
        }
        Rule::Digits => {
            if !value.chars().all(|c| c.is_ascii_digit()) {
                out.push(issue("digits", "non-digit character".to_string()));
            }
        }
        Rule::Ascii => {
            if !value.is_ascii() {
                out.push(issue("ascii", "non-ASCII character".to_string()));
            }
        }
        Rule::Lowercase => {
            if value.chars().any(|c| c.is_uppercase()) {
                out.push(issue("lowercase", "contains uppercase".to_string()));
            }
        }
        Rule::Uppercase => {
            if value.chars().any(|c| c.is_lowercase()) {
                out.push(issue("uppercase", "contains lowercase".to_string()));
            }
        }
        Rule::StartsWith(s) => {
            if !value.starts_with(s.as_str()) {
                out.push(issue("starts_with", format!("does not start with {s:?}")));
            }
        }
        Rule::EndsWith(s) => {
            if !value.ends_with(s.as_str()) {
                out.push(issue("ends_with", format!("does not end with {s:?}")));
            }
        }
        Rule::Contains(s) => {
            if !value.contains(s.as_str()) {
                out.push(issue("contains", format!("does not contain {s:?}")));
            }
        }
        Rule::OneOf(opts) => {
            if !opts.iter().any(|o| o == value) {
                out.push(issue("one_of", format!("not one of {opts:?}")));
            }
        }
        Rule::NotOneOf(opts) => {
            if opts.iter().any(|o| o == value) {
                out.push(issue("not_one_of", format!("value is in {opts:?}")));
            }
        }
        Rule::Integer => {
            if crate::coerce::parse_i64(value).is_err() {
                out.push(issue("integer", "not an integer".to_string()));
            }
        }
        Rule::PositiveInteger => match crate::coerce::parse_i64(value) {
            Ok(n) if n > 0 => {}
            Ok(_) => out.push(issue("positive_integer", "must be > 0".to_string())),
            Err(_) => out.push(issue("positive_integer", "not an integer".to_string())),
        },
        Rule::NonNegativeInteger => match crate::coerce::parse_i64(value) {
            Ok(n) if n >= 0 => {}
            Ok(_) => out.push(issue("non_negative_integer", "must be >= 0".to_string())),
            Err(_) => out.push(issue("non_negative_integer", "not an integer".to_string())),
        },
        Rule::Float => {
            if crate::coerce::parse_f64(value).is_err() {
                out.push(issue("float", "not a float".to_string()));
            }
        }
        Rule::PositiveFloat => match crate::coerce::parse_f64(value) {
            Ok(n) if n > 0.0 => {}
            Ok(_) => out.push(issue("positive_float", "must be > 0".to_string())),
            Err(_) => out.push(issue("positive_float", "not a float".to_string())),
        },
        Rule::NonNegativeFloat => match crate::coerce::parse_f64(value) {
            Ok(n) if n >= 0.0 => {}
            Ok(_) => out.push(issue("non_negative_float", "must be >= 0".to_string())),
            Err(_) => out.push(issue("non_negative_float", "not a float".to_string())),
        },
        Rule::IntRange(min, max) => match crate::coerce::parse_i64(value) {
            Ok(n) if n >= *min && n <= *max => {}
            Ok(n) => out.push(issue(
                "int_range",
                format!("{} not in [{},{}]", n, min, max),
            )),
            Err(_) => out.push(issue("int_range", "not an integer".to_string())),
        },
        Rule::FloatRange(min, max) => match crate::coerce::parse_f64(value) {
            Ok(n) if n >= *min && n <= *max => {}
            Ok(n) => out.push(issue(
                "float_range",
                format!("{} not in [{},{}]", n, min, max),
            )),
            Err(_) => out.push(issue("float_range", "not a float".to_string())),
        },
        Rule::Port => {
            if let Err(e) = crate::coerce::parse_port_u16(value) {
                out.push(issue("port", e));
            }
        }
        Rule::Boolean => {
            if crate::coerce::parse_bool(value).is_err() {
                out.push(issue("boolean", "not a boolean".to_string()));
            }
        }
        Rule::Email => {
            if let Err(e) = check_email(value) {
                out.push(issue("email", e));
            }
        }
        Rule::Url => {
            if let Err(e) = check_url(value) {
                out.push(issue("url", e));
            }
        }
        Rule::Uuid => {
            if let Err(e) = check_uuid(value) {
                out.push(issue("uuid", e));
            }
        }
        Rule::Ipv4 => {
            if Ipv4Addr::from_str(value).is_err() {
                out.push(issue("ipv4", "not an IPv4 address".to_string()));
            }
        }
        Rule::Ip => {
            if IpAddr::from_str(value).is_err() {
                out.push(issue("ip", "not an IP address".to_string()));
            }
        }
        Rule::Hex => {
            if let Err(e) = check_hex(value) {
                out.push(issue("hex", e));
            }
        }
        Rule::List {
            delim,
            item,
            min_items,
            max_items,
        } => {
            let parts: Vec<&str> = value.split(*delim).map(str::trim).collect();
            if let Some(n) = min_items {
                if parts.len() < *n {
                    out.push(issue(
                        "min_items",
                        format!("length {} < {}", parts.len(), n),
                    ));
                }
            }
            if let Some(n) = max_items {
                if parts.len() > *n {
                    out.push(issue(
                        "max_items",
                        format!("length {} > {}", parts.len(), n),
                    ));
                }
            }
            for (i, part) in parts.iter().enumerate() {
                let sub_key = format!("{}[{}]", key, i);
                validate_field(&sub_key, item, Some(*part), out);
            }
        }
    }
}

fn check_email(v: &str) -> Result<(), String> {
    if v.chars().any(char::is_whitespace) {
        return Err("contains whitespace".to_string());
    }
    let mut parts = v.split('@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    if parts.next().is_some() {
        return Err("must contain exactly one '@'".to_string());
    }
    if local.is_empty() {
        return Err("empty local part".to_string());
    }
    if domain.is_empty() {
        return Err("empty domain".to_string());
    }
    Ok(())
}

fn check_url(v: &str) -> Result<(), String> {
    let rest = if let Some(r) = v.strip_prefix("http://") {
        r
    } else if let Some(r) = v.strip_prefix("https://") {
        r
    } else {
        return Err("must start with http:// or https://".to_string());
    };
    if rest.is_empty() {
        return Err("empty host".to_string());
    }
    if rest.chars().any(char::is_whitespace) {
        return Err("contains whitespace".to_string());
    }
    Ok(())
}

fn check_uuid(v: &str) -> Result<(), String> {
    if v.len() != 36 {
        return Err(format!("length {} != 36", v.len()));
    }
    for (i, b) in v.as_bytes().iter().enumerate() {
        let is_dash = matches!(i, 8 | 13 | 18 | 23);
        if is_dash {
            if *b != b'-' {
                return Err(format!("expected '-' at position {}", i));
            }
        } else if !(*b as char).is_ascii_hexdigit() {
            return Err(format!("non-hex digit at position {}", i));
        }
    }
    Ok(())
}

fn check_hex(v: &str) -> Result<(), String> {
    let rest = v
        .strip_prefix("0x")
        .or_else(|| v.strip_prefix("0X"))
        .unwrap_or(v);
    if rest.is_empty() {
        return Err("empty".to_string());
    }
    if !rest.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("non-hex digit".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> EnvroVars {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn issues(res: Result<(), EnvroError>) -> Vec<ValidationIssue> {
        match res {
            Ok(()) => Vec::new(),
            Err(EnvroError::Validation { errors }) => errors,
            Err(e) => panic!("unexpected error variant: {e}"),
        }
    }

    #[test]
    fn required_missing_and_empty_fails() {
        let schema = Schema::new()
            .field("A", Field::required())
            .field("B", Field::required());
        let v = vars(&[("B", "")]);
        let iss = issues(validate(&v, &schema));
        assert_eq!(iss.len(), 2);
        assert!(iss.iter().all(|i| i.rule == "required"));
    }

    #[test]
    fn optional_missing_or_empty_skips_rules() {
        let schema = Schema::new()
            .field("A", Field::optional().min_len(3))
            .field("B", Field::optional().min_len(3));
        let v = vars(&[("B", "")]);
        assert!(validate(&v, &schema).is_ok());
    }

    #[test]
    fn min_max_exact_len() {
        let schema = Schema::new()
            .field("A", Field::required().min_len(3))
            .field("B", Field::required().max_len(2))
            .field("C", Field::required().exact_len(4));
        let v = vars(&[("A", "ab"), ("B", "abc"), ("C", "abcd")]);
        let iss = issues(validate(&v, &schema));
        assert_eq!(iss.len(), 2);
        assert!(iss.iter().any(|i| i.key == "A" && i.rule == "min_len"));
        assert!(iss.iter().any(|i| i.key == "B" && i.rule == "max_len"));
    }

    #[test]
    fn string_shape_rules() {
        let schema = Schema::new()
            .field("A", Field::required().alpha())
            .field("B", Field::required().alphanumeric())
            .field("C", Field::required().digits())
            .field("D", Field::required().ascii())
            .field("E", Field::required().lowercase())
            .field("F", Field::required().uppercase());
        let v = vars(&[
            ("A", "abc"),
            ("B", "abc123"),
            ("C", "123"),
            ("D", "hello"),
            ("E", "abc"),
            ("F", "ABC"),
        ]);
        assert!(validate(&v, &schema).is_ok());

        let bad = vars(&[
            ("A", "ab1"),
            ("B", "ab!"),
            ("C", "12a"),
            ("D", "cafÉ"),
            ("E", "aB"),
            ("F", "Ab"),
        ]);
        let iss = issues(validate(&bad, &schema));
        assert_eq!(iss.len(), 6);
    }

    #[test]
    fn starts_ends_contains_one_of() {
        let schema = Schema::new()
            .field("A", Field::required().starts_with("http"))
            .field("B", Field::required().ends_with(".env"))
            .field("C", Field::required().contains("://"))
            .field(
                "D",
                Field::required().one_of(&["debug", "info", "warn", "error"]),
            )
            .field("E", Field::required().not_one_of(&["root", "admin"]));
        let ok = vars(&[
            ("A", "https://x"),
            ("B", "prod.env"),
            ("C", "pg://x"),
            ("D", "info"),
            ("E", "app"),
        ]);
        assert!(validate(&ok, &schema).is_ok());

        let bad = vars(&[
            ("A", "ftp://x"),
            ("B", "prod.yaml"),
            ("C", "no-scheme"),
            ("D", "trace"),
            ("E", "root"),
        ]);
        let iss = issues(validate(&bad, &schema));
        assert_eq!(iss.len(), 5);
    }

    #[test]
    fn number_rules() {
        let schema = Schema::new()
            .field("I", Field::required().integer())
            .field("P", Field::required().positive_integer())
            .field("N", Field::required().non_negative_integer())
            .field("F", Field::required().float())
            .field("R", Field::required().int_range(10, 20))
            .field("FR", Field::required().float_range(0.0, 1.0));
        let ok = vars(&[
            ("I", "-3"),
            ("P", "1"),
            ("N", "0"),
            ("F", "1.5"),
            ("R", "15"),
            ("FR", "0.5"),
        ]);
        assert!(validate(&ok, &schema).is_ok());

        let bad = vars(&[
            ("I", "x"),
            ("P", "0"),
            ("N", "-1"),
            ("F", "abc"),
            ("R", "99"),
            ("FR", "2.0"),
        ]);
        let iss = issues(validate(&bad, &schema));
        assert_eq!(iss.len(), 6);
    }

    #[test]
    fn port_rule() {
        let schema = Schema::new().field("P", Field::required().port());
        assert!(validate(&vars(&[("P", "8080")]), &schema).is_ok());
        assert!(validate(&vars(&[("P", "1")]), &schema).is_ok());
        assert!(validate(&vars(&[("P", "65535")]), &schema).is_ok());
        assert_eq!(issues(validate(&vars(&[("P", "0")]), &schema)).len(), 1);
        assert_eq!(issues(validate(&vars(&[("P", "70000")]), &schema)).len(), 1);
        assert_eq!(issues(validate(&vars(&[("P", "x")]), &schema)).len(), 1);
    }

    #[test]
    fn boolean_rule() {
        let schema = Schema::new().field("B", Field::required().boolean());
        for good in ["true", "TRUE", "false", "1", "0", "yes", "No"] {
            assert!(
                validate(&vars(&[("B", good)]), &schema).is_ok(),
                "expected {good} to pass"
            );
        }
        for bad in ["truthy", "on", "off", "2"] {
            let iss = issues(validate(&vars(&[("B", bad)]), &schema));
            assert_eq!(iss.len(), 1, "expected {bad} to fail");
            assert_eq!(iss[0].rule, "boolean");
        }
    }

    #[test]
    fn email_url_uuid_ip_hex() {
        let schema = Schema::new()
            .field("E", Field::required().email())
            .field("U", Field::required().url())
            .field("ID", Field::required().uuid())
            .field("V4", Field::required().ipv4())
            .field("IP", Field::required().ip())
            .field("H", Field::required().hex());
        let ok = vars(&[
            ("E", "a@b.co"),
            ("U", "https://example.com/path"),
            ("ID", "550e8400-e29b-41d4-a716-446655440000"),
            ("V4", "192.168.1.1"),
            ("IP", "::1"),
            ("H", "0xdeadBEEF"),
        ]);
        assert!(validate(&ok, &schema).is_ok());

        let bad = vars(&[
            ("E", "no-at"),
            ("U", "ftp://x"),
            ("ID", "not-a-uuid"),
            ("V4", "999.1.1.1"),
            ("IP", "not-ip"),
            ("H", "0xzz"),
        ]);
        let iss = issues(validate(&bad, &schema));
        assert_eq!(iss.len(), 6);
    }

    #[test]
    fn list_applies_item_rules() {
        let schema = Schema::new().field(
            "TAGS",
            Field::required().list(',', Field::required().min_len(2)),
        );
        assert!(validate(&vars(&[("TAGS", "aa, bb, ccc")]), &schema).is_ok());

        let iss = issues(validate(&vars(&[("TAGS", "aa,,x")]), &schema));
        // two failures: empty part (required) + "x" too short
        assert_eq!(iss.len(), 2);
        assert!(iss
            .iter()
            .any(|i| i.key == "TAGS[1]" && i.rule == "required"));
        assert!(iss
            .iter()
            .any(|i| i.key == "TAGS[2]" && i.rule == "min_len"));
    }

    #[test]
    fn list_optional_items_skip_empty() {
        let schema = Schema::new().field(
            "T",
            Field::required().list(',', Field::optional().min_len(2)),
        );
        // "a," -> ["a", ""] ; "a" fails min_len, "" is optional/skipped
        let iss = issues(validate(&vars(&[("T", "a,")]), &schema));
        assert_eq!(iss.len(), 1);
        assert_eq!(iss[0].key, "T[0]");
        assert_eq!(iss[0].rule, "min_len");
    }

    #[test]
    fn list_min_max_items() {
        let schema = Schema::new().field(
            "T",
            Field::required()
                .list(',', Field::required())
                .min_items(2)
                .max_items(3),
        );
        assert!(validate(&vars(&[("T", "a,b")]), &schema).is_ok());
        assert!(validate(&vars(&[("T", "a,b,c")]), &schema).is_ok());
        assert_eq!(issues(validate(&vars(&[("T", "a")]), &schema)).len(), 1);
        assert_eq!(
            issues(validate(&vars(&[("T", "a,b,c,d")]), &schema)).len(),
            1
        );
    }

    #[test]
    fn collects_all_failures() {
        let schema = Schema::new()
            .field("A", Field::required().min_len(5).max_len(3))
            .field("B", Field::required().integer());
        // "abcd" (len 4) fails both min_len(5) and max_len(3); "nope" fails integer
        let iss = issues(validate(&vars(&[("A", "abcd"), ("B", "nope")]), &schema));
        assert_eq!(iss.len(), 3);
        assert!(iss.iter().any(|i| i.key == "A" && i.rule == "min_len"));
        assert!(iss.iter().any(|i| i.key == "A" && i.rule == "max_len"));
        assert!(iss.iter().any(|i| i.key == "B" && i.rule == "integer"));
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let schema = Schema::new().field("A", Field::required());
        let v = vars(&[("A", "x"), ("EXTRA", "y")]);
        assert!(validate(&v, &schema).is_ok());
    }

    #[test]
    fn display_includes_validation_prefix_and_issues() {
        let schema = Schema::new().field("A", Field::required().integer());
        let err = validate(&vars(&[("A", "nope")]), &schema).unwrap_err();
        let s = err.to_string();
        assert!(s.starts_with("VALIDATION_ERROR "), "got: {s}");
        assert!(s.contains("A[integer]"), "got: {s}");
    }
    #[test]
    fn exact_len_fail() {
        let schema = Schema::new().field("A", Field::required().exact_len(4));
        let iss = issues(validate(&vars(&[("A", "xyz")]), &schema));
        assert_eq!(iss.len(), 1);
        assert_eq!(iss[0].rule, "exact_len");
    }

    #[test]
    fn number_parse_error_branches() {
        // Non-numeric input hits the Err arm of positive_integer,
        // non_negative_integer, int_range, positive_float, non_negative_float,
        // float_range.
        let schema = Schema::new()
            .field("PI", Field::required().positive_integer())
            .field("NNI", Field::required().non_negative_integer())
            .field("IR", Field::required().int_range(0, 10))
            .field("PF", Field::required().positive_float())
            .field("NNF", Field::required().non_negative_float())
            .field("FR", Field::required().float_range(0.0, 1.0));
        let iss = issues(validate(
            &vars(&[
                ("PI", "x"),
                ("NNI", "x"),
                ("IR", "x"),
                ("PF", "x"),
                ("NNF", "x"),
                ("FR", "x"),
            ]),
            &schema,
        ));
        assert_eq!(iss.len(), 6);
        assert!(iss
            .iter()
            .any(|i| i.key == "PI" && i.rule == "positive_integer"));
        assert!(iss
            .iter()
            .any(|i| i.key == "NNI" && i.rule == "non_negative_integer"));
        assert!(iss.iter().any(|i| i.key == "IR" && i.rule == "int_range"));
        assert!(iss
            .iter()
            .any(|i| i.key == "PF" && i.rule == "positive_float"));
        assert!(iss
            .iter()
            .any(|i| i.key == "NNF" && i.rule == "non_negative_float"));
        assert!(iss.iter().any(|i| i.key == "FR" && i.rule == "float_range"));
    }

    #[test]
    fn positive_float_and_non_negative_float_value_branches() {
        // Covers positive_float(<=0), non_negative_float(<0), and their happy paths.
        let schema = Schema::new()
            .field("PF", Field::required().positive_float())
            .field("NNF", Field::required().non_negative_float());
        let iss = issues(validate(&vars(&[("PF", "0.0"), ("NNF", "-0.5")]), &schema));
        assert_eq!(iss.len(), 2);
        assert!(iss.iter().any(|i| i.rule == "positive_float"));
        assert!(iss.iter().any(|i| i.rule == "non_negative_float"));

        assert!(validate(&vars(&[("PF", "0.5"), ("NNF", "0.0")]), &schema).is_ok());
    }

    #[test]
    fn email_error_branches() {
        let schema = Schema::new().field("E", Field::required().email());
        let iss = issues(validate(&vars(&[("E", "a b@c.co")]), &schema));
        assert_eq!(iss.len(), 1);
        assert!(iss[0].reason.contains("whitespace"));
        let iss = issues(validate(&vars(&[("E", "a@b@c")]), &schema));
        assert_eq!(iss.len(), 1);
        assert!(iss[0].reason.contains("exactly one"));
        let iss = issues(validate(&vars(&[("E", "@b.co")]), &schema));
        assert_eq!(iss.len(), 1);
        assert!(iss[0].reason.contains("empty local"));
    }

    #[test]
    fn url_http_and_error_branches() {
        let schema = Schema::new().field("U", Field::required().url());
        assert!(validate(&vars(&[("U", "http://x")]), &schema).is_ok());
        let iss = issues(validate(&vars(&[("U", "https://")]), &schema));
        assert_eq!(iss.len(), 1);
        assert!(iss[0].reason.contains("empty host"));
        let iss = issues(validate(&vars(&[("U", "https://a b")]), &schema));
        assert_eq!(iss.len(), 1);
        assert!(iss[0].reason.contains("whitespace"));
    }

    #[test]
    fn uuid_position_and_hex_error_branches() {
        let schema = Schema::new().field("ID", Field::required().uuid());
        // len 36 but non-'-' at position 8
        let iss = issues(validate(
            &vars(&[("ID", "550e8400xe29b-41d4-a716-446655440000")]),
            &schema,
        ));
        assert_eq!(iss.len(), 1);
        assert!(iss[0].reason.contains("expected '-'"));

        // len 36, correct dashes, but non-hex at end
        let iss = issues(validate(
            &vars(&[("ID", "550e8400-e29b-41d4-a716-44665544000z")]),
            &schema,
        ));
        assert_eq!(iss.len(), 1);
        assert!(iss[0].reason.contains("non-hex digit"));
    }

    #[test]
    fn hex_empty_after_prefix_and_uppercase_prefix() {
        let schema = Schema::new().field("H", Field::optional().hex());
        // "0x" strips to "" -> empty branch
        let iss = issues(validate(&vars(&[("H", "0x")]), &schema));
        assert_eq!(iss.len(), 1);
        assert_eq!(iss[0].reason, "empty");
        // exercises the "0X" prefix branch too
        assert!(validate(&vars(&[("H", "0XFF")]), &schema).is_ok());
    }

    #[test]
    fn validate_env_reads_process_env() {
        use serial_test::serial;
        // guard: keep this scope thread-safe.
        #[serial]
        fn inner() {
            std::env::remove_var("VE_A");
            std::env::remove_var("VE_B");
            std::env::remove_var("VE_C");

            let schema = Schema::new()
                .field("VE_A", Field::required().min_len(3))
                .field("VE_B", Field::optional().integer())
                .field("VE_C", Field::required().port());

            // all missing: only required fields fail.
            let err = validate_env(&schema).unwrap_err();
            match err {
                EnvroError::Validation { errors } => {
                    assert_eq!(errors.len(), 2);
                    assert!(errors
                        .iter()
                        .any(|i| i.key == "VE_A" && i.rule == "required"));
                    assert!(errors
                        .iter()
                        .any(|i| i.key == "VE_C" && i.rule == "required"));
                }
                other => panic!("expected Validation, got {other}"),
            }

            // set valid values -> ok.
            std::env::set_var("VE_A", "envro");
            std::env::set_var("VE_C", "8080");
            validate_env(&schema).unwrap();

            // add a bad optional -> single failure.
            std::env::set_var("VE_B", "not-int");
            let err = validate_env(&schema).unwrap_err();
            match err {
                EnvroError::Validation { errors } => {
                    assert_eq!(errors.len(), 1);
                    assert_eq!(errors[0].key, "VE_B");
                    assert_eq!(errors[0].rule, "integer");
                }
                other => panic!("expected Validation, got {other}"),
            }

            std::env::remove_var("VE_A");
            std::env::remove_var("VE_B");
            std::env::remove_var("VE_C");
        }
        inner();
    }

    #[test]
    fn load_dotenv_validated_ok_and_err() {
        use std::io::Write;

        let dir = std::env::temp_dir();
        let ok_file = dir.join(".env-validate-ok");
        let mut f = std::fs::File::create(&ok_file).unwrap();
        f.write_all(b"NAME=envro\nPORT=8080").unwrap();
        let schema = Schema::new()
            .field("NAME", Field::required().min_len(3))
            .field("PORT", Field::required().port());
        let loaded = load_dotenv_validated(ok_file.as_path(), &schema).unwrap();
        assert_eq!(loaded.get("NAME").map(String::as_str), Some("envro"));
        assert_eq!(loaded.get("PORT").map(String::as_str), Some("8080"));

        let bad_file = dir.join(".env-validate-bad");
        let mut f = std::fs::File::create(&bad_file).unwrap();
        f.write_all(b"NAME=ab\nPORT=nope").unwrap();
        let err = load_dotenv_validated(bad_file.as_path(), &schema).unwrap_err();
        match err {
            EnvroError::Validation { errors } => assert_eq!(errors.len(), 2),
            other => panic!("expected Validation, got {other}"),
        }

        // Missing file -> EnvroError::File propagated.
        let err = load_dotenv_validated(Path::new("/nonexistent/.env-x"), &schema).unwrap_err();
        assert!(matches!(err, EnvroError::File { .. }));
    }
}
