//! Age encryption helpers (feature `encryption`).
//!
//! Marker format: `Encrypted[AGE:b64:<standard-base64>]` — see docs/encryption.md.

use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine};

use crate::{EnvroError, EnvroVars};

pub(crate) const IDENTITY_FILE_KEY: &str = "ENVRO_AGE_IDENTITY_FILE";

const MARKER_PREFIX: &str = "Encrypted[AGE:b64:";
const MARKER_SUFFIX: char = ']';

pub(crate) fn is_encrypted_marker(value: &str) -> bool {
    value.starts_with(MARKER_PREFIX) && value.ends_with(MARKER_SUFFIX)
}

fn crypto_err(key: impl Into<String>, reason: impl ToString) -> EnvroError {
    EnvroError::Decrypt {
        key: key.into(),
        reason: reason.to_string(),
    }
}

/// Encrypt plaintext to one or more age recipients → `Encrypted[AGE:b64:…]`.
pub fn encrypt_value<R: age::Recipient>(
    plaintext: &str,
    recipients: &[R],
) -> Result<String, EnvroError> {
    if recipients.is_empty() {
        return Err(crypto_err("(encrypt)", "missing recipients"));
    }
    let ciphertext = if recipients.len() == 1 {
        age::encrypt(&recipients[0], plaintext.as_bytes())
            .map_err(|e| crypto_err("(encrypt)", e))?
    } else {
        let encryptor =
            age::Encryptor::with_recipients(recipients.iter().map(|r| r as &dyn age::Recipient))
                .map_err(|e| crypto_err("(encrypt)", e))?;
        let mut out = Vec::with_capacity(plaintext.len() + 128);
        let mut writer = encryptor.wrap_output(&mut out).expect("vec wrap_output");
        writer
            .write_all(plaintext.as_bytes())
            .expect("vec write_all");
        writer.finish().expect("vec finish");
        out
    };
    Ok(format!(
        "{MARKER_PREFIX}{}{MARKER_SUFFIX}",
        STANDARD.encode(ciphertext)
    ))
}

/// Decrypt one marked value, or return `value` unchanged if not `Encrypted[…]`.
pub fn decrypt_value<I: age::Identity>(
    value: &str,
    identities: &[I],
) -> Result<String, EnvroError> {
    decrypt_with_dyn(
        value,
        identities.iter().map(|i| i as &dyn age::Identity),
        "(decrypt)",
    )
}

fn decrypt_with_dyn<'a>(
    value: &str,
    identities: impl Iterator<Item = &'a dyn age::Identity>,
    key: &str,
) -> Result<String, EnvroError> {
    if !is_encrypted_marker(value) {
        return Ok(value.to_string());
    }

    let b64 = &value[MARKER_PREFIX.len()..value.len() - 1];
    let ciphertext = STANDARD
        .decode(b64)
        .map_err(|e| crypto_err(key, format!("invalid base64: {e}")))?;

    let ids: Vec<&dyn age::Identity> = identities.collect();
    let decryptor =
        age::Decryptor::new_buffered(&ciphertext[..]).map_err(|e| crypto_err(key, e))?;
    let mut reader = decryptor
        .decrypt(ids.into_iter())
        .map_err(|e| crypto_err(key, e))?;
    let mut plaintext = Vec::new();
    reader
        .read_to_end(&mut plaintext)
        .map_err(|e| crypto_err(key, e))?;

    String::from_utf8(plaintext).map_err(|e| crypto_err(key, format!("plaintext not utf-8: {e}")))
}

pub(crate) fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home);
        }
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

fn load_identities(path: &Path) -> Result<Vec<Box<dyn age::Identity>>, EnvroError> {
    let path_str = path.to_string_lossy().into_owned();
    let file = age::IdentityFile::from_file(path_str.clone()).map_err(|e| {
        crypto_err(
            IDENTITY_FILE_KEY,
            format!("cannot read identity file {path_str}: {e}"),
        )
    })?;
    let identities = file
        .into_identities()
        .map_err(|e| crypto_err(IDENTITY_FILE_KEY, e))?;
    if identities.is_empty() {
        return Err(crypto_err(
            IDENTITY_FILE_KEY,
            format!("no identities in {path_str}"),
        ));
    }
    Ok(identities)
}

/// Strip `ENVRO_AGE_IDENTITY_FILE`, decrypt every `Encrypted[…]` value.
///
/// Identity material is dropped (and zeroized by age) when this returns.
pub(crate) fn decrypt_dotenv_vars(mut vars: EnvroVars) -> Result<EnvroVars, EnvroError> {
    let identity_path = vars.remove(IDENTITY_FILE_KEY);
    let encrypted_keys: Vec<String> = vars
        .iter()
        .filter(|(_, v)| is_encrypted_marker(v))
        .map(|(k, _)| k.clone())
        .collect();

    if encrypted_keys.is_empty() {
        return Ok(vars);
    }

    let path = identity_path.ok_or_else(|| {
        crypto_err(
            IDENTITY_FILE_KEY,
            "missing ENVRO_AGE_IDENTITY_FILE for Encrypted values",
        )
    })?;
    let path = expand_tilde(path.trim());
    let identities = load_identities(&path)?;
    let id_refs: Vec<&dyn age::Identity> = identities.iter().map(|b| b.as_ref()).collect();

    for key in encrypted_keys {
        let value = vars.get(&key).expect("key from scan").clone();
        let plain = decrypt_with_dyn(&value, id_refs.iter().copied(), &key)?;
        vars.insert(key, plain);
    }

    drop(identities);
    Ok(vars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn expand_tilde_home_and_relative() {
        let _g = env_lock().lock().unwrap();
        let home = env::var("HOME").expect("HOME");
        assert_eq!(expand_tilde("~"), PathBuf::from(&home));
        assert_eq!(
            expand_tilde("~/foo/bar"),
            PathBuf::from(&home).join("foo/bar")
        );
        assert_eq!(expand_tilde("/abs/path"), PathBuf::from("/abs/path"));
    }

    #[test]
    fn expand_tilde_without_home_falls_back_to_literal() {
        let _g = env_lock().lock().unwrap();
        let prev = env::var_os("HOME");
        env::remove_var("HOME");
        assert_eq!(expand_tilde("~"), PathBuf::from("~"));
        assert_eq!(expand_tilde("~/x"), PathBuf::from("~/x"));
        match prev {
            Some(v) => env::set_var("HOME", v),
            None => env::remove_var("HOME"),
        }
    }
}
