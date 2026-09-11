//! Parse env strings into Rust primitives (shared by validation and `#[derive(Envro)]`).

use crate::{EnvroError, EnvroVars, ValidationIssue};

/// Runtime tagged value for coerced env vars (primitives + string).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Bool(bool),
    I32(i32),
    I64(i64),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
}

fn issue(key: &str, rule: &'static str, reason: impl Into<String>) -> EnvroError {
    EnvroError::Validation {
        errors: vec![ValidationIssue {
            key: key.to_string(),
            rule,
            reason: reason.into(),
        }],
    }
}

/// Same accept list as [`crate::Field::boolean`].
pub fn parse_bool(value: &str) -> Result<bool, &'static str> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("true") || value == "1" || value.eq_ignore_ascii_case("yes") {
        Ok(true)
    } else if value.eq_ignore_ascii_case("false")
        || value == "0"
        || value.eq_ignore_ascii_case("no")
    {
        Ok(false)
    } else {
        Err("not a boolean")
    }
}

pub fn parse_i32(value: &str) -> Result<i32, &'static str> {
    value.trim().parse().map_err(|_| "not an i32")
}

pub fn parse_i64(value: &str) -> Result<i64, &'static str> {
    value.trim().parse().map_err(|_| "not an integer")
}

pub fn parse_u16(value: &str) -> Result<u16, &'static str> {
    value.trim().parse().map_err(|_| "not a u16")
}

pub fn parse_u32(value: &str) -> Result<u32, &'static str> {
    value.trim().parse().map_err(|_| "not a u32")
}

pub fn parse_u64(value: &str) -> Result<u64, &'static str> {
    value.trim().parse().map_err(|_| "not a u64")
}

pub fn parse_f32(value: &str) -> Result<f32, &'static str> {
    let n: f32 = value.trim().parse().map_err(|_| "not a float")?;
    if n.is_finite() {
        Ok(n)
    } else {
        Err("not a finite float")
    }
}

pub fn parse_f64(value: &str) -> Result<f64, &'static str> {
    let n: f64 = value.trim().parse().map_err(|_| "not a float")?;
    if n.is_finite() {
        Ok(n)
    } else {
        Err("not a finite float")
    }
}

/// Port range check used by [`crate::Field::port`].
pub fn parse_port_u16(value: &str) -> Result<u16, String> {
    match value.trim().parse::<u32>() {
        Ok(n) if (1..=65535).contains(&n) => Ok(n as u16),
        Ok(n) => Err(format!("{n} not in 1..=65535")),
        Err(_) => Err("not a number".to_string()),
    }
}

fn lookup<'a>(vars: &'a EnvroVars, key: &str) -> Option<&'a str> {
    vars.get(key).map(String::as_str).filter(|v| !v.is_empty())
}

pub fn require_string(vars: &EnvroVars, key: &str) -> Result<String, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Err(issue(key, "required", "value is missing or empty"));
    };
    Ok(raw.to_string())
}

pub fn optional_string(vars: &EnvroVars, key: &str) -> Result<Option<String>, EnvroError> {
    Ok(lookup(vars, key).map(str::to_string))
}

pub fn require_bool(vars: &EnvroVars, key: &str) -> Result<bool, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Err(issue(key, "required", "value is missing or empty"));
    };
    parse_bool(raw).map_err(|reason| issue(key, "boolean", reason))
}

pub fn optional_bool(vars: &EnvroVars, key: &str) -> Result<Option<bool>, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Ok(None);
    };
    parse_bool(raw)
        .map(Some)
        .map_err(|reason| issue(key, "boolean", reason))
}

pub fn require_i32(vars: &EnvroVars, key: &str) -> Result<i32, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Err(issue(key, "required", "value is missing or empty"));
    };
    parse_i32(raw).map_err(|reason| issue(key, "i32", reason))
}

pub fn optional_i32(vars: &EnvroVars, key: &str) -> Result<Option<i32>, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Ok(None);
    };
    parse_i32(raw)
        .map(Some)
        .map_err(|reason| issue(key, "i32", reason))
}

pub fn require_i64(vars: &EnvroVars, key: &str) -> Result<i64, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Err(issue(key, "required", "value is missing or empty"));
    };
    parse_i64(raw).map_err(|reason| issue(key, "integer", reason))
}

pub fn optional_i64(vars: &EnvroVars, key: &str) -> Result<Option<i64>, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Ok(None);
    };
    parse_i64(raw)
        .map(Some)
        .map_err(|reason| issue(key, "integer", reason))
}

pub fn require_u16(vars: &EnvroVars, key: &str) -> Result<u16, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Err(issue(key, "required", "value is missing or empty"));
    };
    parse_u16(raw).map_err(|reason| issue(key, "u16", reason))
}

pub fn optional_u16(vars: &EnvroVars, key: &str) -> Result<Option<u16>, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Ok(None);
    };
    parse_u16(raw)
        .map(Some)
        .map_err(|reason| issue(key, "u16", reason))
}

pub fn require_u32(vars: &EnvroVars, key: &str) -> Result<u32, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Err(issue(key, "required", "value is missing or empty"));
    };
    parse_u32(raw).map_err(|reason| issue(key, "u32", reason))
}

pub fn optional_u32(vars: &EnvroVars, key: &str) -> Result<Option<u32>, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Ok(None);
    };
    parse_u32(raw)
        .map(Some)
        .map_err(|reason| issue(key, "u32", reason))
}

pub fn require_u64(vars: &EnvroVars, key: &str) -> Result<u64, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Err(issue(key, "required", "value is missing or empty"));
    };
    parse_u64(raw).map_err(|reason| issue(key, "u64", reason))
}

pub fn optional_u64(vars: &EnvroVars, key: &str) -> Result<Option<u64>, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Ok(None);
    };
    parse_u64(raw)
        .map(Some)
        .map_err(|reason| issue(key, "u64", reason))
}

pub fn require_f32(vars: &EnvroVars, key: &str) -> Result<f32, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Err(issue(key, "required", "value is missing or empty"));
    };
    parse_f32(raw).map_err(|reason| issue(key, "float", reason))
}

pub fn optional_f32(vars: &EnvroVars, key: &str) -> Result<Option<f32>, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Ok(None);
    };
    parse_f32(raw)
        .map(Some)
        .map_err(|reason| issue(key, "float", reason))
}

pub fn require_f64(vars: &EnvroVars, key: &str) -> Result<f64, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Err(issue(key, "required", "value is missing or empty"));
    };
    parse_f64(raw).map_err(|reason| issue(key, "float", reason))
}

pub fn optional_f64(vars: &EnvroVars, key: &str) -> Result<Option<f64>, EnvroError> {
    let Some(raw) = lookup(vars, key) else {
        return Ok(None);
    };
    parse_f64(raw)
        .map(Some)
        .map_err(|reason| issue(key, "float", reason))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_dialect() {
        assert!(parse_bool("YES").unwrap());
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(!parse_bool("no").unwrap());
        assert!(!parse_bool("FALSE").unwrap());
        assert!(parse_bool("on").is_err());
    }

    #[test]
    fn port_range() {
        assert_eq!(parse_port_u16("8080").unwrap(), 8080);
        assert!(parse_port_u16("0").is_err());
        assert!(parse_port_u16("70000").is_err());
        assert!(parse_port_u16("abc").is_err());
    }

    #[test]
    fn parse_primitives_ok_and_err() {
        assert_eq!(parse_i32("-3").unwrap(), -3);
        assert!(parse_i32("x").is_err());
        assert_eq!(parse_i64("42").unwrap(), 42);
        assert!(parse_i64("x").is_err());
        assert_eq!(parse_u16("1").unwrap(), 1);
        assert!(parse_u16("-1").is_err());
        assert_eq!(parse_u32("2").unwrap(), 2);
        assert!(parse_u32("x").is_err());
        assert_eq!(parse_u64("3").unwrap(), 3);
        assert!(parse_u64("x").is_err());
        assert_eq!(parse_f32("1.5").unwrap(), 1.5);
        assert!(parse_f32("x").is_err());
        assert_eq!(parse_f64("2.5").unwrap(), 2.5);
        assert!(parse_f64("x").is_err());
    }

    #[test]
    fn value_variants_roundtrip_eq() {
        assert_eq!(Value::Str("a".into()), Value::Str("a".into()));
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::I32(1), Value::I32(1));
        assert_eq!(Value::I64(2), Value::I64(2));
        assert_eq!(Value::U16(3), Value::U16(3));
        assert_eq!(Value::U32(4), Value::U32(4));
        assert_eq!(Value::U64(5), Value::U64(5));
        assert_eq!(Value::F32(1.0), Value::F32(1.0));
        assert_eq!(Value::F64(2.0), Value::F64(2.0));
        let _ = format!("{:?}", Value::Str("x".into()));
        let _ = Value::Bool(false).clone();
    }

    #[test]
    fn require_and_optional_all_types() {
        let mut vars = EnvroVars::new();
        vars.insert("S".into(), "hi".into());
        vars.insert("B".into(), "yes".into());
        vars.insert("I32".into(), "-1".into());
        vars.insert("I64".into(), "9".into());
        vars.insert("U16".into(), "8080".into());
        vars.insert("U32".into(), "10".into());
        vars.insert("U64".into(), "11".into());
        vars.insert("F32".into(), "1.25".into());
        vars.insert("F64".into(), "2.5".into());
        vars.insert("BAD_B".into(), "on".into());
        vars.insert("BAD_N".into(), "nope".into());
        vars.insert("EMPTY".into(), "".into());

        assert_eq!(require_string(&vars, "S").unwrap(), "hi");
        assert_eq!(optional_string(&vars, "S").unwrap().as_deref(), Some("hi"));
        assert_eq!(optional_string(&vars, "MISSING").unwrap(), None);
        assert_eq!(optional_string(&vars, "EMPTY").unwrap(), None);
        assert!(require_string(&vars, "MISSING").is_err());
        assert!(require_string(&vars, "EMPTY").is_err());

        assert!(require_bool(&vars, "B").unwrap());
        assert_eq!(optional_bool(&vars, "B").unwrap(), Some(true));
        assert_eq!(optional_bool(&vars, "MISSING").unwrap(), None);
        assert!(require_bool(&vars, "MISSING").is_err());
        assert!(require_bool(&vars, "BAD_B").is_err());
        assert!(optional_bool(&vars, "BAD_B").is_err());

        assert_eq!(require_i32(&vars, "I32").unwrap(), -1);
        assert_eq!(optional_i32(&vars, "I32").unwrap(), Some(-1));
        assert_eq!(optional_i32(&vars, "MISSING").unwrap(), None);
        assert!(require_i32(&vars, "MISSING").is_err());
        assert!(require_i32(&vars, "BAD_N").is_err());
        assert!(optional_i32(&vars, "BAD_N").is_err());

        assert_eq!(require_i64(&vars, "I64").unwrap(), 9);
        assert_eq!(optional_i64(&vars, "I64").unwrap(), Some(9));
        assert_eq!(optional_i64(&vars, "MISSING").unwrap(), None);
        assert!(require_i64(&vars, "MISSING").is_err());
        assert!(require_i64(&vars, "BAD_N").is_err());
        assert!(optional_i64(&vars, "BAD_N").is_err());

        assert_eq!(require_u16(&vars, "U16").unwrap(), 8080);
        assert_eq!(optional_u16(&vars, "U16").unwrap(), Some(8080));
        assert_eq!(optional_u16(&vars, "MISSING").unwrap(), None);
        assert!(require_u16(&vars, "MISSING").is_err());
        assert!(require_u16(&vars, "BAD_N").is_err());
        assert!(optional_u16(&vars, "BAD_N").is_err());

        assert_eq!(require_u32(&vars, "U32").unwrap(), 10);
        assert_eq!(optional_u32(&vars, "U32").unwrap(), Some(10));
        assert_eq!(optional_u32(&vars, "MISSING").unwrap(), None);
        assert!(require_u32(&vars, "MISSING").is_err());
        assert!(require_u32(&vars, "BAD_N").is_err());
        assert!(optional_u32(&vars, "BAD_N").is_err());

        assert_eq!(require_u64(&vars, "U64").unwrap(), 11);
        assert_eq!(optional_u64(&vars, "U64").unwrap(), Some(11));
        assert_eq!(optional_u64(&vars, "MISSING").unwrap(), None);
        assert!(require_u64(&vars, "MISSING").is_err());
        assert!(require_u64(&vars, "BAD_N").is_err());
        assert!(optional_u64(&vars, "BAD_N").is_err());

        assert_eq!(require_f32(&vars, "F32").unwrap(), 1.25);
        assert_eq!(optional_f32(&vars, "F32").unwrap(), Some(1.25));
        assert_eq!(optional_f32(&vars, "MISSING").unwrap(), None);
        assert!(require_f32(&vars, "MISSING").is_err());
        assert!(require_f32(&vars, "BAD_N").is_err());
        assert!(optional_f32(&vars, "BAD_N").is_err());

        assert_eq!(require_f64(&vars, "F64").unwrap(), 2.5);
        assert_eq!(optional_f64(&vars, "F64").unwrap(), Some(2.5));
        assert_eq!(optional_f64(&vars, "MISSING").unwrap(), None);
        assert!(require_f64(&vars, "MISSING").is_err());
        assert!(require_f64(&vars, "BAD_N").is_err());
        assert!(optional_f64(&vars, "BAD_N").is_err());
    }
}
