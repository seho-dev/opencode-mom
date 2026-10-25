#![allow(dead_code)]

use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub struct GeneratedKey {
    pub raw_token: String,
    pub verifier: String,
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn from_hex(hex: &str) -> Option<Vec<u8>> {
    if hex.len() & 1 != 0 || !hex.is_ascii() {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

pub fn hash_token(token: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hasher.finalize().into()
}

pub fn generate_key() -> GeneratedKey {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let raw_token = to_hex(&bytes);
    let hash = hash_token(&raw_token);
    let verifier = to_hex(&hash);
    GeneratedKey {
        raw_token,
        verifier,
    }
}

pub fn verify_token(incoming_token: &str, expected_verifier_hex: &str) -> bool {
    let incoming_hash = hash_token(incoming_token);
    let Some(expected_hash) = from_hex(expected_verifier_hex) else {
        return false;
    };
    if expected_hash.len() != 32 {
        return false;
    }
    incoming_hash.ct_eq(expected_hash.as_slice()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_generation_and_verification() {
        let key = generate_key();
        assert_eq!(key.raw_token.len(), 64);
        assert_eq!(key.verifier.len(), 64);
        assert!(verify_token(&key.raw_token, &key.verifier));
        assert!(!verify_token("invalid-token", &key.verifier));
        assert!(!verify_token(&key.raw_token, "invalid-verifier"));
        assert!(!verify_token(&key.raw_token, "aéa"));
    }
}
