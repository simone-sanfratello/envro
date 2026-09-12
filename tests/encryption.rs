//! Age encryption tests (feature `encryption`).
//!
//! TDD red: these express the contract in docs/encryption.md and docs/validation.md.
//! They fail to compile or fail at runtime until the implementation lands.
//!
//! ```bash
//! cargo test --features encryption --test encryption
//! ```

#![cfg(feature = "encryption")]

use std::fs;
use std::path::{Path, PathBuf};

use envro::{
    decrypt_value, encrypt_value, load_dotenv, Envro, EnvroConfig, EnvroError, EnvroVars, Field,
    Schema,
};

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "envro-enc-{}-{}-{}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Generate a classic age identity file for round-trip tests.
/// Production docs prefer `age-keygen -pq`; the age crate round-trips with X25519 here.
fn write_test_identity(path: &Path) -> age::x25519::Identity {
    use age::secrecy::ExposeSecret;

    let id = age::x25519::Identity::generate();
    fs::write(path, format!("{}\n", id.to_string().expose_secret())).unwrap();
    id
}

#[test]
fn encrypt_value_round_trips_via_decrypt_value() {
    let id = age::x25519::Identity::generate();
    let enc = encrypt_value("s3cr3t-pass", &[id.to_public()]).unwrap();
    assert!(
        enc.starts_with("Encrypted[AGE:b64:") && enc.ends_with(']'),
        "got {enc}"
    );

    let plain = decrypt_value(&enc, &[id]).unwrap();
    assert_eq!(plain, "s3cr3t-pass");
}

#[test]
fn decrypt_value_leaves_plaintext_unchanged() {
    let id = age::x25519::Identity::generate();
    let out = decrypt_value("localhost", &[id]).unwrap();
    assert_eq!(out, "localhost");
}

#[test]
fn load_dotenv_decrypts_encrypted_and_expands_vars() {
    let dir = tmp_dir("load");
    let key_path = dir.join("example.key");
    let id = write_test_identity(&key_path);

    let enc = encrypt_value("s3cr3t1", &[id.to_public()]).unwrap();

    let env_path = dir.join(".env");
    fs::write(
        &env_path,
        format!(
            "ENVRO_AGE_IDENTITY_FILE={}\n\
             PG_HOST=localhost\n\
             PG_PASS={enc}\n\
             DATABASE_URI=pg://app:${{PG_PASS}}@${{PG_HOST}}/app\n",
            key_path.display()
        ),
    )
    .unwrap();

    let vars = load_dotenv(&env_path).unwrap();
    assert_eq!(vars.get("PG_HOST").map(String::as_str), Some("localhost"));
    assert_eq!(vars.get("PG_PASS").map(String::as_str), Some("s3cr3t1"));
    assert_eq!(
        vars.get("DATABASE_URI").map(String::as_str),
        Some("pg://app:s3cr3t1@localhost/app")
    );
    assert!(
        !vars.contains_key("ENVRO_AGE_IDENTITY_FILE"),
        "identity path must be stripped from the result map"
    );
}

#[test]
fn load_dotenv_missing_identity_is_decrypt_error() {
    let dir = tmp_dir("no-id");
    let env_path = dir.join(".env");
    // Valid-looking marker without a resolvable identity
    fs::write(&env_path, "PG_PASS=Encrypted[AGE:b64:dGVzdA==]\n").unwrap();

    let err = load_dotenv(&env_path).unwrap_err();
    assert!(
        matches!(err, EnvroError::Decrypt { .. }),
        "expected Decrypt, got {err}"
    );
}

#[test]
fn load_dotenv_plaintext_keys_unchanged() {
    let dir = tmp_dir("plain");
    let env_path = dir.join(".env");
    fs::write(&env_path, "PG_HOST=localhost\nLOG_LEVEL=info\n").unwrap();

    let vars = load_dotenv(&env_path).unwrap();
    assert_eq!(vars.get("PG_HOST").map(String::as_str), Some("localhost"));
    assert_eq!(vars.get("LOG_LEVEL").map(String::as_str), Some("info"));
}

#[test]
fn secret_field_rejects_plaintext_in_dotenv() {
    let dir = tmp_dir("secret-plain");
    let env_path = dir.join(".env");
    fs::write(
        &env_path,
        "ENVRO_AGE_IDENTITY_FILE=/tmp/missing.key\nPG_PASS=s3cr3t1\n",
    )
    .unwrap();

    let schema = Schema::new().field("PG_PASS", Field::required().secret().min_len(6));

    let err = envro::load_dotenv_validated(&env_path, &schema).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("PG_PASS") || matches!(err, EnvroError::Decrypt { .. }),
        "expected secret/plaintext rejection, got {err}"
    );
}

#[test]
fn secret_field_allows_plaintext_in_from_vars_process_style() {
    // CI / from_env path: values are already plain in a map
    let schema = Schema::new().field("PG_PASS", Field::required().secret().min_len(6));
    let mut vars = EnvroVars::new();
    vars.insert("PG_PASS".into(), "s3cr3t1".into());
    envro::validate(&vars, &schema).unwrap();
}

#[test]
fn secret_derive_from_vars_allows_ci_plaintext() {
    #[derive(Debug, Envro, PartialEq)]
    struct Config {
        #[envro(from = "PG_PASS", secret, min_len = 6)]
        pg_pass: String,
    }

    let mut vars = EnvroVars::new();
    vars.insert("PG_PASS".into(), "s3cr3t1".into());
    let cfg = Config::from_vars(&vars).unwrap();
    assert_eq!(cfg.pg_pass, "s3cr3t1");
}

#[test]
fn multi_recipient_either_identity_decrypts() {
    let alice = age::x25519::Identity::generate();
    let bob = age::x25519::Identity::generate();
    let enc = encrypt_value("shared-secret", &[alice.to_public(), bob.to_public()]).unwrap();

    assert_eq!(decrypt_value(&enc, &[alice]).unwrap(), "shared-secret");
    assert_eq!(decrypt_value(&enc, &[bob]).unwrap(), "shared-secret");
}

#[test]
fn encrypt_value_rejects_empty_recipients() {
    let err = encrypt_value("x", &[] as &[age::x25519::Recipient]).unwrap_err();
    assert!(matches!(err, EnvroError::Decrypt { .. }), "got {err}");
}

#[test]
fn decrypt_value_rejects_invalid_base64() {
    let id = age::x25519::Identity::generate();
    let err = decrypt_value("Encrypted[AGE:b64:!!!]", &[id]).unwrap_err();
    assert!(matches!(err, EnvroError::Decrypt { .. }), "got {err}");
}

#[test]
fn decrypt_value_rejects_wrong_identity() {
    let alice = age::x25519::Identity::generate();
    let bob = age::x25519::Identity::generate();
    let enc = encrypt_value("secret", &[alice.to_public()]).unwrap();
    let err = decrypt_value(&enc, &[bob]).unwrap_err();
    assert!(matches!(err, EnvroError::Decrypt { .. }), "got {err}");
}

#[test]
fn decrypt_value_rejects_non_utf8_plaintext() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let id = age::x25519::Identity::generate();
    let raw = age::encrypt(&id.to_public(), &[0xff, 0xfe, 0xfd]).unwrap();
    let enc = format!("Encrypted[AGE:b64:{}]", STANDARD.encode(raw));
    let err = decrypt_value(&enc, &[id]).unwrap_err();
    assert!(matches!(err, EnvroError::Decrypt { .. }), "got {err}");
}

#[test]
fn load_dotenv_expands_tilde_in_identity_path() {
    use age::secrecy::ExposeSecret;
    let dir = tmp_dir("tilde");
    // Point HOME at our temp dir so `~/…` is writable in CI/sandbox.
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &dir);

    let key_path = dir.join("id.key");
    let id = age::x25519::Identity::generate();
    fs::write(&key_path, format!("{}\n", id.to_string().expose_secret())).unwrap();

    let enc = encrypt_value("tilde-secret", &[id.to_public()]).unwrap();
    let env_path = dir.join(".env");
    fs::write(
        &env_path,
        format!("ENVRO_AGE_IDENTITY_FILE=~/id.key\nPG_PASS={enc}\n"),
    )
    .unwrap();

    let vars = load_dotenv(&env_path).unwrap();
    assert_eq!(
        vars.get("PG_PASS").map(String::as_str),
        Some("tilde-secret")
    );

    match prev_home {
        Some(v) => std::env::set_var("HOME", v),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn load_dotenv_strips_identity_key_without_ciphertext() {
    let dir = tmp_dir("strip");
    let env_path = dir.join(".env");
    fs::write(
        &env_path,
        "ENVRO_AGE_IDENTITY_FILE=/tmp/unused.key\nPG_HOST=localhost\n",
    )
    .unwrap();
    let vars = load_dotenv(&env_path).unwrap();
    assert_eq!(vars.get("PG_HOST").map(String::as_str), Some("localhost"));
    assert!(!vars.contains_key("ENVRO_AGE_IDENTITY_FILE"));
}

#[test]
fn load_dotenv_missing_identity_file_is_decrypt_error() {
    let dir = tmp_dir("missing-key");
    let id = age::x25519::Identity::generate();
    let enc = encrypt_value("x", &[id.to_public()]).unwrap();
    let env_path = dir.join(".env");
    fs::write(
        &env_path,
        format!(
            "ENVRO_AGE_IDENTITY_FILE={}/no-such.key\nPG_PASS={enc}\n",
            dir.display()
        ),
    )
    .unwrap();
    let err = load_dotenv(&env_path).unwrap_err();
    assert!(matches!(err, EnvroError::Decrypt { .. }), "got {err}");
}

#[test]
fn load_dotenv_empty_identity_file_is_decrypt_error() {
    let dir = tmp_dir("empty-key");
    let key_path = dir.join("empty.key");
    fs::write(&key_path, "# only a comment\n\n").unwrap();
    let id = age::x25519::Identity::generate();
    let enc = encrypt_value("x", &[id.to_public()]).unwrap();
    let env_path = dir.join(".env");
    fs::write(
        &env_path,
        format!(
            "ENVRO_AGE_IDENTITY_FILE={}\nPG_PASS={enc}\n",
            key_path.display()
        ),
    )
    .unwrap();
    let err = load_dotenv(&env_path).unwrap_err();
    assert!(matches!(err, EnvroError::Decrypt { .. }), "got {err}");
}

#[test]
fn load_dotenv_accepts_age_keygen_style_identity_file() {
    use age::secrecy::ExposeSecret;
    let dir = tmp_dir("keygen-style");
    let key_path = dir.join("example.key");
    let id = age::x25519::Identity::generate();
    fs::write(
        &key_path,
        format!(
            "# created: 2026-01-01\n# public key: {}\n{}\n",
            id.to_public(),
            id.to_string().expose_secret()
        ),
    )
    .unwrap();
    let enc = encrypt_value("styled", &[id.to_public()]).unwrap();
    let env_path = dir.join(".env");
    fs::write(
        &env_path,
        format!(
            "ENVRO_AGE_IDENTITY_FILE={}\nPG_PASS={enc}\n",
            key_path.display()
        ),
    )
    .unwrap();
    let vars = load_dotenv(&env_path).unwrap();
    assert_eq!(vars.get("PG_PASS").map(String::as_str), Some("styled"));
}

#[test]
fn decrypt_error_display_includes_key() {
    let err = EnvroError::Decrypt {
        key: "PG_PASS".into(),
        reason: "boom".into(),
    };
    let s = err.to_string();
    assert!(s.contains("PG_PASS") && s.contains("boom"), "{s}");
}

#[test]
fn decrypt_value_rejects_corrupt_ciphertext() {
    let id = age::x25519::Identity::generate();
    // valid base64, not an age header
    let err = decrypt_value("Encrypted[AGE:b64:dGVzdA==]", &[id]).unwrap_err();
    assert!(matches!(err, EnvroError::Decrypt { .. }), "got {err}");
}

#[test]
fn secret_field_missing_key_skips_marker_check_then_fails_required() {
    let dir = tmp_dir("secret-missing");
    let env_path = dir.join(".env");
    fs::write(&env_path, "OTHER=1\n").unwrap();
    let schema = Schema::new().field("PG_PASS", Field::required().secret().min_len(1));
    let err = envro::load_dotenv_validated(&env_path, &schema).unwrap_err();
    assert!(
        matches!(err, EnvroError::Validation { .. }),
        "expected Validation, got {err}"
    );
}

#[test]
fn secret_field_empty_value_skips_marker_check_then_fails_required() {
    let dir = tmp_dir("secret-empty");
    let env_path = dir.join(".env");
    fs::write(&env_path, "PG_PASS=\n").unwrap();
    let schema = Schema::new().field("PG_PASS", Field::required().secret().min_len(1));
    let err = envro::load_dotenv_validated(&env_path, &schema).unwrap_err();
    assert!(
        matches!(err, EnvroError::Validation { .. }),
        "expected Validation, got {err}"
    );
}
