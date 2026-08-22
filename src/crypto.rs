//! Ported almost verbatim from the Tauri reference app's `src-tauri/src/crypto.rs`.
//! Only the `#[tauri::command]` attributes were removed and `serde` derives dropped
//! (no longer needed without IPC serialization) — the logic, error messages, and all
//! unit tests are unchanged.

use aes::Aes128;
use aes::Aes192;
use aes::Aes256;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use cbc::{Decryptor, Encryptor};
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use des::Des;
use rand::RngCore;

#[derive(Debug)]
pub struct CryptoError {
    pub message: String,
}

#[derive(Debug)]
pub struct CbcResult {
    pub cipher: String,
    pub iv: String,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

fn err(msg: &str) -> CryptoError {
    CryptoError { message: msg.to_string() }
}

// Alphanumeric-only strings (generated keys) are treated as plain text.
// A string is treated as Base64 only when it contains +, / or = — chars that
// generated keys never include but real Base64 output always has.
pub fn looks_like_base64(s: &str) -> bool {
    s.contains('+') || s.contains('/') || s.contains('=')
}

pub fn hash_md5(plaintext: String) -> String {
    let digest = md5::compute(plaintext.as_bytes());
    format!("{:x}", digest)
}

pub fn hash_md5_bytes(data: &[u8]) -> String {
    let digest = md5::compute(data);
    format!("{:x}", digest)
}

pub fn encrypt_aes_cbc(plaintext: String, key: String, iv: Option<String>) -> Result<CbcResult, CryptoError> {
    let key_bytes = key.as_bytes();
    let mut iv_buf = [0u8; 16];
    match iv {
        Some(ref s) if !s.is_empty() => {
            let bytes = if looks_like_base64(s) {
                BASE64.decode(s).map_err(|_| err("IV is not valid Base64"))?
            } else {
                s.as_bytes().to_vec()
            };
            if bytes.len() != 16 { return Err(err("AES IV must be exactly 16 bytes")); }
            iv_buf.copy_from_slice(&bytes);
        }
        _ => rand::thread_rng().fill_bytes(&mut iv_buf),
    }
    let ciphertext = match key_bytes.len() {
        16 => Encryptor::<Aes128>::new_from_slices(key_bytes, &iv_buf)
                .map_err(|_| err("Invalid key length"))?.encrypt_padded_vec_mut::<Pkcs7>(plaintext.as_bytes()),
        24 => Encryptor::<Aes192>::new_from_slices(key_bytes, &iv_buf)
                .map_err(|_| err("Invalid key length"))?.encrypt_padded_vec_mut::<Pkcs7>(plaintext.as_bytes()),
        32 => Encryptor::<Aes256>::new_from_slices(key_bytes, &iv_buf)
                .map_err(|_| err("Invalid key length"))?.encrypt_padded_vec_mut::<Pkcs7>(plaintext.as_bytes()),
        _ => return Err(err("AES key must be 16, 24, or 32 bytes")),
    };
    Ok(CbcResult { cipher: BASE64.encode(ciphertext), iv: BASE64.encode(iv_buf) })
}

pub fn decrypt_aes_cbc(payload: String, key: String, iv: String) -> Result<String, CryptoError> {
    let key_bytes = key.as_bytes();
    let iv_bytes = if looks_like_base64(&iv) {
        BASE64.decode(&iv).map_err(|_| err("IV is not valid Base64"))?
    } else {
        iv.as_bytes().to_vec()
    };
    let cipher_bytes = BASE64.decode(&payload).map_err(|_| err("Invalid cipher Base64"))?;
    let plaintext = match key_bytes.len() {
        16 => Decryptor::<Aes128>::new_from_slices(key_bytes, &iv_bytes)
                .map_err(|_| err("Invalid key/IV length"))?.decrypt_padded_vec_mut::<Pkcs7>(&cipher_bytes)
                .map_err(|_| err("Decryption failed: invalid key or corrupted data"))?,
        24 => Decryptor::<Aes192>::new_from_slices(key_bytes, &iv_bytes)
                .map_err(|_| err("Invalid key/IV length"))?.decrypt_padded_vec_mut::<Pkcs7>(&cipher_bytes)
                .map_err(|_| err("Decryption failed: invalid key or corrupted data"))?,
        32 => Decryptor::<Aes256>::new_from_slices(key_bytes, &iv_bytes)
                .map_err(|_| err("Invalid key/IV length"))?.decrypt_padded_vec_mut::<Pkcs7>(&cipher_bytes)
                .map_err(|_| err("Decryption failed: invalid key or corrupted data"))?,
        _ => return Err(err("AES key must be 16, 24, or 32 bytes")),
    };
    String::from_utf8(plaintext).map_err(|_| err("Decrypted data is not valid UTF-8"))
}

type AesEcbEnc128 = ecb::Encryptor<Aes128>;
type AesEcbEnc192 = ecb::Encryptor<Aes192>;
type AesEcbEnc256 = ecb::Encryptor<Aes256>;
type AesEcbDec128 = ecb::Decryptor<Aes128>;
type AesEcbDec192 = ecb::Decryptor<Aes192>;
type AesEcbDec256 = ecb::Decryptor<Aes256>;

pub fn encrypt_aes_ecb(plaintext: String, key: String) -> Result<String, CryptoError> {
    use cipher::KeyInit;
    let key_bytes = key.as_bytes();
    let plaintext_bytes = plaintext.as_bytes();

    let ciphertext = match key_bytes.len() {
        16 => {
            let enc = AesEcbEnc128::new_from_slice(key_bytes)
                .map_err(|_| err("Invalid key length"))?;
            enc.encrypt_padded_vec_mut::<Pkcs7>(plaintext_bytes)
        }
        24 => {
            let enc = AesEcbEnc192::new_from_slice(key_bytes)
                .map_err(|_| err("Invalid key length"))?;
            enc.encrypt_padded_vec_mut::<Pkcs7>(plaintext_bytes)
        }
        32 => {
            let enc = AesEcbEnc256::new_from_slice(key_bytes)
                .map_err(|_| err("Invalid key length"))?;
            enc.encrypt_padded_vec_mut::<Pkcs7>(plaintext_bytes)
        }
        _ => return Err(err("AES key must be 16, 24, or 32 bytes")),
    };

    Ok(BASE64.encode(ciphertext))
}

pub fn decrypt_aes_ecb(payload: String, key: String) -> Result<String, CryptoError> {
    use cipher::KeyInit;
    let key_bytes = key.as_bytes();
    let cipher_bytes = BASE64.decode(&payload).map_err(|_| err("Invalid base64 payload"))?;

    let plaintext = match key_bytes.len() {
        16 => {
            let dec = AesEcbDec128::new_from_slice(key_bytes)
                .map_err(|_| err("Invalid key length"))?;
            dec.decrypt_padded_vec_mut::<Pkcs7>(&cipher_bytes)
                .map_err(|_| err("Decryption failed: invalid key or corrupted data"))?
        }
        24 => {
            let dec = AesEcbDec192::new_from_slice(key_bytes)
                .map_err(|_| err("Invalid key length"))?;
            dec.decrypt_padded_vec_mut::<Pkcs7>(&cipher_bytes)
                .map_err(|_| err("Decryption failed: invalid key or corrupted data"))?
        }
        32 => {
            let dec = AesEcbDec256::new_from_slice(key_bytes)
                .map_err(|_| err("Invalid key length"))?;
            dec.decrypt_padded_vec_mut::<Pkcs7>(&cipher_bytes)
                .map_err(|_| err("Decryption failed: invalid key or corrupted data"))?
        }
        _ => return Err(err("AES key must be 16, 24, or 32 bytes")),
    };

    String::from_utf8(plaintext).map_err(|_| err("Decrypted data is not valid UTF-8"))
}

type DesEcbEnc = ecb::Encryptor<Des>;
type DesEcbDec = ecb::Decryptor<Des>;
type DesCbcEnc = Encryptor<Des>;
type DesCbcDec = Decryptor<Des>;

pub fn encrypt_des_ecb(plaintext: String, key: String) -> Result<String, CryptoError> {
    use cipher::KeyInit;
    let key_bytes = key.as_bytes();
    if key_bytes.len() != 8 {
        return Err(err("DES key must be exactly 8 bytes"));
    }
    let enc = DesEcbEnc::new_from_slice(key_bytes)
        .map_err(|_| err("Invalid key length"))?;
    let ciphertext = enc.encrypt_padded_vec_mut::<Pkcs7>(plaintext.as_bytes());
    Ok(BASE64.encode(ciphertext))
}

pub fn decrypt_des_ecb(payload: String, key: String) -> Result<String, CryptoError> {
    use cipher::KeyInit;
    let key_bytes = key.as_bytes();
    if key_bytes.len() != 8 {
        return Err(err("DES key must be exactly 8 bytes"));
    }
    let cipher_bytes = BASE64.decode(&payload).map_err(|_| err("Invalid base64 payload"))?;
    let dec = DesEcbDec::new_from_slice(key_bytes)
        .map_err(|_| err("Invalid key length"))?;
    let plaintext = dec.decrypt_padded_vec_mut::<Pkcs7>(&cipher_bytes)
        .map_err(|_| err("Decryption failed: invalid key or corrupted data"))?;
    String::from_utf8(plaintext).map_err(|_| err("Decrypted data is not valid UTF-8"))
}

pub fn encrypt_des_cbc(plaintext: String, key: String, iv: Option<String>) -> Result<CbcResult, CryptoError> {
    let key_bytes = key.as_bytes();
    if key_bytes.len() != 8 { return Err(err("DES key must be exactly 8 bytes")); }
    let mut iv_buf = [0u8; 8];
    match iv {
        Some(ref s) if !s.is_empty() => {
            let bytes = if looks_like_base64(s) {
                BASE64.decode(s).map_err(|_| err("IV is not valid Base64"))?
            } else {
                s.as_bytes().to_vec()
            };
            if bytes.len() != 8 { return Err(err("DES IV must be exactly 8 bytes")); }
            iv_buf.copy_from_slice(&bytes);
        }
        _ => rand::thread_rng().fill_bytes(&mut iv_buf),
    }
    let enc: DesCbcEnc = DesCbcEnc::new_from_slices(key_bytes, &iv_buf)
        .map_err(|_| err("Invalid key/IV length"))?;
    let ciphertext = enc.encrypt_padded_vec_mut::<Pkcs7>(plaintext.as_bytes());
    Ok(CbcResult { cipher: BASE64.encode(ciphertext), iv: BASE64.encode(iv_buf) })
}

pub fn decrypt_des_cbc(payload: String, key: String, iv: String) -> Result<String, CryptoError> {
    let key_bytes = key.as_bytes();
    if key_bytes.len() != 8 { return Err(err("DES key must be exactly 8 bytes")); }
    let iv_bytes = if looks_like_base64(&iv) {
        BASE64.decode(&iv).map_err(|_| err("IV is not valid Base64"))?
    } else {
        iv.as_bytes().to_vec()
    };
    let cipher_bytes = BASE64.decode(&payload).map_err(|_| err("Invalid cipher Base64"))?;
    let dec: DesCbcDec = DesCbcDec::new_from_slices(key_bytes, &iv_bytes)
        .map_err(|_| err("Invalid key/IV length"))?;
    let plaintext = dec.decrypt_padded_vec_mut::<Pkcs7>(&cipher_bytes)
        .map_err(|_| err("Decryption failed: invalid key or corrupted data"))?;
    String::from_utf8(plaintext).map_err(|_| err("Decrypted data is not valid UTF-8"))
}

pub fn generate_key(bits: u32) -> Result<String, CryptoError> {
    let length = (bits / 8) as usize;
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let mut key = String::with_capacity(length);
    for _ in 0..length {
        let mut byte = [0u8; 1];
        loop {
            rng.fill_bytes(&mut byte);
            if (byte[0] as usize) < ALPHABET.len() * (256 / ALPHABET.len()) {
                key.push(ALPHABET[byte[0] as usize % ALPHABET.len()] as char);
                break;
            }
        }
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── MD5 ──────────────────────────────────────────────────────────────────

    #[test]
    fn md5_known_vector() {
        assert_eq!(hash_md5("hello".to_string()), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn md5_empty_string() {
        assert_eq!(hash_md5("".to_string()), "d41d8cd98f00b204e9800998ecf8427e");
    }

    // ── AES-CBC ───────────────────────────────────────────────────────────────

    #[test]
    fn aes_cbc_roundtrip_128() {
        let key = "0123456789abcdef".to_string();
        let plaintext = "Hello, AES-128-CBC!".to_string();
        let res = encrypt_aes_cbc(plaintext.clone(), key.clone(), None).unwrap();
        let decrypted = decrypt_aes_cbc(res.cipher, key, res.iv).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_cbc_roundtrip_192() {
        let key = "0123456789abcdef01234567".to_string();
        let plaintext = "Hello, AES-192-CBC!".to_string();
        let res = encrypt_aes_cbc(plaintext.clone(), key.clone(), None).unwrap();
        let decrypted = decrypt_aes_cbc(res.cipher, key, res.iv).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_cbc_roundtrip_256() {
        let key = "0123456789abcdef0123456789abcdef".to_string();
        let plaintext = "Hello, AES-256-CBC!".to_string();
        let res = encrypt_aes_cbc(plaintext.clone(), key.clone(), None).unwrap();
        let decrypted = decrypt_aes_cbc(res.cipher, key, res.iv).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_cbc_custom_iv_plain_roundtrip() {
        let key = "0123456789abcdef".to_string();
        let plaintext = "Custom IV test!".to_string();
        let iv = "abcdefghijklmnop".to_string();
        let res = encrypt_aes_cbc(plaintext.clone(), key.clone(), Some(iv.clone())).unwrap();
        let decrypted = decrypt_aes_cbc(res.cipher, key, iv).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_cbc_custom_iv_plain_deterministic() {
        let key = "0123456789abcdef".to_string();
        let plaintext = "Same IV same output".to_string();
        let iv = "abcdefghijklmnop".to_string();
        let r1 = encrypt_aes_cbc(plaintext.clone(), key.clone(), Some(iv.clone())).unwrap();
        let r2 = encrypt_aes_cbc(plaintext.clone(), key.clone(), Some(iv)).unwrap();
        assert_eq!(r1.cipher, r2.cipher, "same IV must produce same ciphertext");
    }

    #[test]
    fn aes_cbc_custom_iv_base64_roundtrip() {
        let key = "0123456789abcdef".to_string();
        let plaintext = "Base64 IV test!".to_string();
        let iv_b64 = BASE64.encode(b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f");
        let res = encrypt_aes_cbc(plaintext.clone(), key.clone(), Some(iv_b64.clone())).unwrap();
        let decrypted = decrypt_aes_cbc(res.cipher, key, iv_b64).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_cbc_custom_iv_wrong_length() {
        let key = "0123456789abcdef".to_string();
        let err = encrypt_aes_cbc("text".to_string(), key, Some("tooshort".to_string())).unwrap_err();
        assert!(err.message.contains("16 bytes"));
    }

    #[test]
    fn aes_cbc_custom_iv_invalid_base64() {
        let key = "0123456789abcdef".to_string();
        let err = encrypt_aes_cbc("text".to_string(), key, Some("not=valid=b64!!".to_string())).unwrap_err();
        assert!(err.message.contains("Base64"));
    }

    #[test]
    fn aes_cbc_wrong_key_length() {
        let err = encrypt_aes_cbc("text".to_string(), "short".to_string(), None).unwrap_err();
        assert!(err.message.contains("16, 24, or 32"));
    }

    #[test]
    fn aes_cbc_wrong_key_on_decrypt() {
        let key = "0123456789abcdef".to_string();
        let res = encrypt_aes_cbc("secret".to_string(), key, None).unwrap();
        let result = decrypt_aes_cbc(res.cipher, "fedcba9876543210".to_string(), res.iv);
        assert!(result.is_err());
    }

    #[test]
    fn aes_cbc_invalid_cipher_base64() {
        let err = decrypt_aes_cbc("not!base64".to_string(), "0123456789abcdef".to_string(), "AAAAAAAAAAAAAAAAAAAAAA==".to_string()).unwrap_err();
        assert!(err.message.contains("Base64"));
    }

    // ── AES-ECB ───────────────────────────────────────────────────────────────

    #[test]
    fn aes_ecb_roundtrip_128() {
        let key = "0123456789abcdef".to_string();
        let plaintext = "Hello, AES-128-ECB!".to_string();
        let ciphertext = encrypt_aes_ecb(plaintext.clone(), key.clone()).unwrap();
        let decrypted = decrypt_aes_ecb(ciphertext, key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_ecb_roundtrip_192() {
        let key = "0123456789abcdef01234567".to_string();
        let plaintext = "Hello, AES-192-ECB!".to_string();
        let ciphertext = encrypt_aes_ecb(plaintext.clone(), key.clone()).unwrap();
        let decrypted = decrypt_aes_ecb(ciphertext, key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_ecb_roundtrip_256() {
        let key = "0123456789abcdef0123456789abcdef".to_string();
        let plaintext = "Hello, AES-256-ECB!".to_string();
        let ciphertext = encrypt_aes_ecb(plaintext.clone(), key.clone()).unwrap();
        let decrypted = decrypt_aes_ecb(ciphertext, key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_ecb_wrong_key_length() {
        let err = encrypt_aes_ecb("text".to_string(), "short".to_string()).unwrap_err();
        assert!(err.message.contains("16, 24, or 32"));
    }

    // ── DES-ECB ───────────────────────────────────────────────────────────────

    #[test]
    fn des_ecb_roundtrip() {
        let key = "8bytekey".to_string(); // 8 bytes
        let plaintext = "Hello, DES-ECB!".to_string();
        let ciphertext = encrypt_des_ecb(plaintext.clone(), key.clone()).unwrap();
        let decrypted = decrypt_des_ecb(ciphertext, key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn des_ecb_wrong_key_length() {
        let err = encrypt_des_ecb("text".to_string(), "short".to_string()).unwrap_err();
        assert!(err.message.contains("8 bytes"));
    }

    #[test]
    fn des_ecb_wrong_key_on_decrypt() {
        let key = "8bytekey".to_string();
        let ciphertext = encrypt_des_ecb("secret".to_string(), key).unwrap();
        let result = decrypt_des_ecb(ciphertext, "otherkey".to_string());
        assert!(result.is_err());
    }

    // ── DES-CBC ───────────────────────────────────────────────────────────────

    #[test]
    fn des_cbc_roundtrip() {
        let key = "8bytekey".to_string();
        let plaintext = "Hello, DES-CBC!".to_string();
        let res = encrypt_des_cbc(plaintext.clone(), key.clone(), None).unwrap();
        let decrypted = decrypt_des_cbc(res.cipher, key, res.iv).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn des_cbc_custom_iv_plain_roundtrip() {
        let key = "8bytekey".to_string();
        let plaintext = "DES custom IV!".to_string();
        let iv = "12345678".to_string();
        let res = encrypt_des_cbc(plaintext.clone(), key.clone(), Some(iv.clone())).unwrap();
        let decrypted = decrypt_des_cbc(res.cipher, key, iv).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn des_cbc_custom_iv_plain_deterministic() {
        let key = "8bytekey".to_string();
        let plaintext = "Same IV same output".to_string();
        let iv = "12345678".to_string();
        let r1 = encrypt_des_cbc(plaintext.clone(), key.clone(), Some(iv.clone())).unwrap();
        let r2 = encrypt_des_cbc(plaintext.clone(), key.clone(), Some(iv)).unwrap();
        assert_eq!(r1.cipher, r2.cipher, "same IV must produce same ciphertext");
    }

    #[test]
    fn des_cbc_custom_iv_base64_roundtrip() {
        let key = "8bytekey".to_string();
        let plaintext = "DES B64 IV!".to_string();
        let iv_b64 = BASE64.encode(b"\x00\x01\x02\x03\x04\x05\x06\x07");
        let res = encrypt_des_cbc(plaintext.clone(), key.clone(), Some(iv_b64.clone())).unwrap();
        let decrypted = decrypt_des_cbc(res.cipher, key, iv_b64).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn des_cbc_custom_iv_wrong_length() {
        let err = encrypt_des_cbc("text".to_string(), "8bytekey".to_string(), Some("toolong16bytes!!".to_string())).unwrap_err();
        assert!(err.message.contains("8 bytes"));
    }

    #[test]
    fn des_cbc_wrong_key_length() {
        let err = encrypt_des_cbc("text".to_string(), "short".to_string(), None).unwrap_err();
        assert!(err.message.contains("8 bytes"));
    }

    #[test]
    fn des_cbc_invalid_cipher_base64() {
        let err = decrypt_des_cbc("not!base64".to_string(), "8bytekey".to_string(), "AAAAAAAAAAA=".to_string()).unwrap_err();
        assert!(err.message.contains("Base64"));
    }

    // ── generate_key ──────────────────────────────────────────────────────────

    #[test]
    fn generate_key_correct_lengths() {
        for bits in [64u32, 128, 192, 256, 512] {
            let key = generate_key(bits).unwrap();
            assert_eq!(key.len(), (bits / 8) as usize, "bits={bits}");
        }
    }

    #[test]
    fn generate_key_only_alphanumeric() {
        let key = generate_key(256).unwrap();
        assert!(key.chars().all(|c| c.is_ascii_alphanumeric()), "non-alphanumeric char in key: {key}");
    }

    #[test]
    fn generate_key_is_random() {
        let a = generate_key(256).unwrap();
        let b = generate_key(256).unwrap();
        assert_ne!(a, b, "two generated keys must not be identical");
    }
}
