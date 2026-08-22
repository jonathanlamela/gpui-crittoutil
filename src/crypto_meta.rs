//! Port of `useCrypto.js`'s algorithm metadata + dispatch + validation helpers.

use crate::crypto;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgId {
    Md5,
    AesCbc,
    AesEcb,
    DesEcb,
    DesCbc,
}

#[derive(Debug, Clone, Copy)]
pub struct AlgMeta {
    pub id: AlgId,
    pub name: &'static str,
    /// Kept for parity with the JS metadata table (`requireIv`); redundant with
    /// `iv_length.is_some()`, which is what the UI actually checks.
    #[allow(dead_code)]
    pub require_iv: bool,
    pub require_key: bool,
    pub key_lengths: &'static [usize],
    pub iv_length: Option<usize>,
}

pub const ENCRYPT_ALGORITHMS: &[AlgMeta] = &[
    AlgMeta { id: AlgId::Md5, name: "MD5", require_iv: false, require_key: false, key_lengths: &[], iv_length: None },
    AlgMeta { id: AlgId::AesCbc, name: "AES (CBC)", require_iv: true, require_key: true, key_lengths: &[16, 24, 32], iv_length: Some(16) },
    AlgMeta { id: AlgId::AesEcb, name: "AES (ECB)", require_iv: false, require_key: true, key_lengths: &[16, 24, 32], iv_length: None },
    AlgMeta { id: AlgId::DesEcb, name: "DES (ECB)", require_iv: false, require_key: true, key_lengths: &[8], iv_length: None },
    AlgMeta { id: AlgId::DesCbc, name: "DES (CBC)", require_iv: true, require_key: true, key_lengths: &[8], iv_length: Some(8) },
];

pub const DECRYPT_ALGORITHMS: &[AlgMeta] = &[
    AlgMeta { id: AlgId::AesCbc, name: "AES (CBC)", require_iv: true, require_key: true, key_lengths: &[16, 24, 32], iv_length: Some(16) },
    AlgMeta { id: AlgId::AesEcb, name: "AES (ECB)", require_iv: false, require_key: true, key_lengths: &[16, 24, 32], iv_length: None },
    AlgMeta { id: AlgId::DesEcb, name: "DES (ECB)", require_iv: false, require_key: true, key_lengths: &[8], iv_length: None },
    AlgMeta { id: AlgId::DesCbc, name: "DES (CBC)", require_iv: true, require_key: true, key_lengths: &[8], iv_length: Some(8) },
];

pub enum EncryptResult {
    Plain(String),
    Cbc { cipher: String, iv: String },
}

pub fn encrypt(alg: &AlgMeta, plaintext: &str, key: &str, iv: Option<&str>) -> Result<EncryptResult, String> {
    match alg.id {
        AlgId::Md5 => Ok(EncryptResult::Plain(crypto::hash_md5(plaintext.to_string()))),
        AlgId::AesCbc => crypto::encrypt_aes_cbc(plaintext.to_string(), key.to_string(), iv.map(|s| s.to_string()))
            .map(|r| EncryptResult::Cbc { cipher: r.cipher, iv: r.iv })
            .map_err(|e| e.message),
        AlgId::AesEcb => crypto::encrypt_aes_ecb(plaintext.to_string(), key.to_string())
            .map(EncryptResult::Plain)
            .map_err(|e| e.message),
        AlgId::DesEcb => crypto::encrypt_des_ecb(plaintext.to_string(), key.to_string())
            .map(EncryptResult::Plain)
            .map_err(|e| e.message),
        AlgId::DesCbc => crypto::encrypt_des_cbc(plaintext.to_string(), key.to_string(), iv.map(|s| s.to_string()))
            .map(|r| EncryptResult::Cbc { cipher: r.cipher, iv: r.iv })
            .map_err(|e| e.message),
    }
}

pub fn decrypt(alg: &AlgMeta, payload: &str, key: &str, iv: Option<&str>) -> Result<String, String> {
    match alg.id {
        AlgId::Md5 => Err("Unknown algorithm".to_string()),
        AlgId::AesCbc => crypto::decrypt_aes_cbc(payload.to_string(), key.to_string(), iv.unwrap_or_default().to_string())
            .map_err(|e| e.message),
        AlgId::AesEcb => crypto::decrypt_aes_ecb(payload.to_string(), key.to_string()).map_err(|e| e.message),
        AlgId::DesEcb => crypto::decrypt_des_ecb(payload.to_string(), key.to_string()).map_err(|e| e.message),
        AlgId::DesCbc => crypto::decrypt_des_cbc(payload.to_string(), key.to_string(), iv.unwrap_or_default().to_string())
            .map_err(|e| e.message),
    }
}

/// Port of `validateKey` / `getKeyError` — byte-length check against `alg.keyLengths`.
pub fn key_error(alg: &AlgMeta, key: &str) -> Option<String> {
    if !alg.require_key {
        return None;
    }
    if key.is_empty() {
        return Some("Please enter a key".to_string());
    }
    let len = key.as_bytes().len();
    if !alg.key_lengths.is_empty() && !alg.key_lengths.contains(&len) {
        return Some(if alg.key_lengths.contains(&8) {
            "Key must be exactly 8 bytes for DES".to_string()
        } else {
            "Key must be 16, 24, or 32 bytes for AES".to_string()
        });
    }
    None
}

/// Port of `getIvError` — base64-vs-plain heuristic using presence of `+`, `/`, `=`.
pub fn iv_error(alg: &AlgMeta, iv: &str) -> Option<String> {
    let iv_length = alg.iv_length?;
    if iv.is_empty() {
        return None;
    }
    let is_b64 = crypto::looks_like_base64(iv);
    if is_b64 {
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
        let decoded = match BASE64.decode(iv) {
            Ok(d) => d,
            Err(_) => return Some("IV is not valid Base64".to_string()),
        };
        if decoded.len() != iv_length {
            return Some(if iv_length == 8 { "IV must be exactly 8 bytes for DES".to_string() } else { "IV must be exactly 16 bytes for AES".to_string() });
        }
    } else {
        let byte_len = iv.as_bytes().len();
        if byte_len != iv_length {
            return Some(if iv_length == 8 { "IV must be exactly 8 bytes for DES".to_string() } else { "IV must be exactly 16 bytes for AES".to_string() });
        }
    }
    None
}

/// Port of decrypter's `getIvError`, which (unlike the encrypter) also requires
/// the field to be non-empty whenever the algorithm needs an IV.
pub fn iv_error_required(alg: &AlgMeta, iv: &str) -> Option<String> {
    if alg.iv_length.is_none() {
        return None;
    }
    if iv.trim().is_empty() {
        return Some("Please enter an IV".to_string());
    }
    iv_error(alg, iv)
}

/// Port of decrypter's `getPayloadError`.
pub fn payload_error(val: &str) -> Option<String> {
    if val.trim().is_empty() {
        return Some("Please enter text to decrypt".to_string());
    }
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    if BASE64.decode(val.trim()).is_err() {
        return Some("Payload is not valid Base64".to_string());
    }
    None
}
