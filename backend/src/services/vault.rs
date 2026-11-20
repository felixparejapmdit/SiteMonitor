//! Encryption at rest for personnel data (workers, every workforce list
//! including the clinic, and workforce snapshots): AES-256-GCM with a random
//! 96-bit nonce per value. Login accounts are not personnel data and are not
//! touched here.
//!
//! The key comes from `SITEMONITOR_DATA_KEY` (64 hex characters) or, failing
//! that, `data/keys/personnel.key`, which is created on first start. Without
//! the key the personnel data cannot be read, so keep a copy of it somewhere
//! safe (not in Git). Moving data between machines goes through encrypted
//! backup packages with their own passphrase, so the key never has to travel.

use std::path::Path;
use std::sync::OnceLock;

use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde_json::Value;

use crate::error::{AppError, AppResult};

const PREFIX: &str = "smv1:";
static KEY: OnceLock<Key<Aes256Gcm>> = OnceLock::new();

fn parse_hex(text: &str) -> Option<Key<Aes256Gcm>> {
    let t = text.trim();
    if t.len() != 64 {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..64).step_by(2).map(|i| u8::from_str_radix(&t[i..i + 2], 16).ok()).collect();
    Some(*Key::<Aes256Gcm>::from_slice(&bytes?))
}

/// Loads (or on first start creates) the personnel data key.
pub fn init(key_file: &Path) -> anyhow::Result<()> {
    let key = match std::env::var("SITEMONITOR_DATA_KEY") {
        Ok(v) if !v.trim().is_empty() => parse_hex(&v).ok_or_else(|| anyhow::anyhow!("SITEMONITOR_DATA_KEY must be 64 hex characters (32 bytes)"))?,
        _ => match std::fs::read_to_string(key_file) {
            Ok(text) => parse_hex(&text).ok_or_else(|| anyhow::anyhow!("{} is not a valid key (64 hex characters)", key_file.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let key = Aes256Gcm::generate_key(OsRng);
                if let Some(dir) = key_file.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
                std::fs::write(key_file, hex + "\n")?;
                tracing::warn!(file = %key_file.display(), "created a new personnel data key: back it up somewhere safe (not in Git)");
                key
            }
            Err(e) => return Err(e.into()),
        },
    };
    let _ = KEY.set(key);
    Ok(())
}

fn cipher() -> AppResult<Aes256Gcm> {
    // Tests and one-off commands that never called `init` get a throwaway key.
    let key = KEY.get_or_init(|| Aes256Gcm::generate_key(OsRng));
    Ok(Aes256Gcm::new(key))
}

/// Encrypts a JSON value into a JSON string: `"smv1:" + base64(nonce ‖ ciphertext ‖ tag)`.
pub fn seal(v: &Value) -> AppResult<Value> {
    let plain = serde_json::to_vec(v).map_err(|e| AppError::bad(e.to_string()))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher()?.encrypt(&nonce, plain.as_ref()).map_err(|_| AppError::bad("Could not encrypt personnel data."))?;
    let mut out = nonce.to_vec();
    out.extend(ct);
    Ok(Value::String(format!("{PREFIX}{}", B64.encode(out))))
}

/// Decrypts a sealed value; anything not sealed (data from before encryption) is returned as is.
pub fn open(v: Value) -> AppResult<Value> {
    let Some(text) = v.as_str().and_then(|s| s.strip_prefix(PREFIX)) else { return Ok(v) };
    let raw = B64.decode(text).map_err(|_| AppError::bad("Stored personnel data is damaged."))?;
    if raw.len() < 12 + 16 {
        return Err(AppError::bad("Stored personnel data is damaged."));
    }
    let (nonce, ct) = raw.split_at(12);
    let plain = cipher()?
        .decrypt(Nonce::from_slice(nonce), ct)
        .map_err(|_| AppError::bad("Personnel data could not be decrypted: this server has a different data key (data/keys/personnel.key or SITEMONITOR_DATA_KEY)."))?;
    serde_json::from_slice(&plain).map_err(|_| AppError::bad("Stored personnel data is damaged."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trip_unique_nonces_tamper_and_plain_passthrough() {
        let v = json!({ "name": "Test Person", "medical": "none" });
        let a = seal(&v).unwrap();
        let b = seal(&v).unwrap();
        assert!(a.as_str().unwrap().starts_with(PREFIX));
        assert_ne!(a, b, "a fresh nonce every time");
        assert!(!a.as_str().unwrap().contains("Test Person"));
        assert_eq!(open(a.clone()).unwrap(), v);
        // Any change to the stored text is caught by the GCM tag.
        let mut s = a.as_str().unwrap().to_string();
        let last = s.pop().unwrap();
        s.push(if last == 'A' { 'B' } else { 'A' });
        assert!(open(Value::String(s)).is_err());
        // Data stored before encryption still reads.
        assert_eq!(open(json!({ "x": 1 })).unwrap(), json!({ "x": 1 }));
    }

    #[test]
    fn hex_keys() {
        assert!(parse_hex(&"ab".repeat(32)).is_some());
        assert!(parse_hex("abc").is_none());
        assert!(parse_hex(&"zz".repeat(32)).is_none());
    }
}
