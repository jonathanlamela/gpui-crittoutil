//! Port of `ConverterView.vue`'s `convert()` / `validateInput()` logic.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvType {
    Text,
    Binary,
    Base64,
}

impl ConvType {
    pub const ALL: [ConvType; 3] = [ConvType::Text, ConvType::Binary, ConvType::Base64];

    pub fn label(&self) -> &'static str {
        match self {
            ConvType::Text => "Text",
            ConvType::Binary => "Binary",
            ConvType::Base64 => "Base64",
        }
    }

    /// The first type in canonical order that isn't `self` — mirrors the Vue
    /// `outputType` auto-adjustment when it collides with the new `inputType`.
    pub fn first_other(&self) -> ConvType {
        ConvType::ALL.into_iter().find(|t| t != self).unwrap()
    }
}

/// Validates `input` for `from_type`. Mirrors `validateInput()` exactly.
pub fn validate_input(input: &str, from_type: ConvType) -> Result<(), String> {
    if input.trim().is_empty() {
        return Err("Please enter a value".to_string());
    }
    match from_type {
        ConvType::Binary => {
            if !input.chars().all(|c| c == '0' || c == '1' || c.is_whitespace()) || input.is_empty() {
                return Err("Value must be binary (0s and 1s)".to_string());
            }
        }
        ConvType::Base64 => {
            if !input.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=') {
                return Err("Value must be valid Base64".to_string());
            }
        }
        ConvType::Text => {}
    }
    Ok(())
}

/// Mirrors `convert()`. Assumes `validate_input` already passed.
pub fn convert(input: &str, from_type: ConvType, to_type: ConvType) -> Result<String, String> {
    let val = input.trim();
    let bytes: Vec<u8> = match from_type {
        ConvType::Text => val.as_bytes().to_vec(),
        ConvType::Binary => {
            let mut out = Vec::new();
            for group in val.split_whitespace() {
                let b = u8::from_str_radix(group, 2).map_err(|_| "Invalid binary group".to_string())?;
                out.push(b);
            }
            out
        }
        ConvType::Base64 => BASE64.decode(val).map_err(|_| "Invalid Base64 input".to_string())?,
    };

    match to_type {
        ConvType::Text => String::from_utf8(bytes).map_err(|_| "Decoded bytes are not valid UTF-8".to_string()),
        ConvType::Binary => Ok(bytes.iter().map(|b| format!("{:08b}", b)).collect::<Vec<_>>().join(" ")),
        ConvType::Base64 => Ok(BASE64.encode(&bytes)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_to_base64() {
        assert_eq!(convert("Hello", ConvType::Text, ConvType::Base64).unwrap(), "SGVsbG8=");
    }

    #[test]
    fn base64_to_text() {
        assert_eq!(convert("SGVsbG8=", ConvType::Base64, ConvType::Text).unwrap(), "Hello");
    }

    #[test]
    fn text_to_binary_and_back() {
        let bin = convert("Hi", ConvType::Text, ConvType::Binary).unwrap();
        assert_eq!(bin, "01001000 01101001");
        assert_eq!(convert(&bin, ConvType::Binary, ConvType::Text).unwrap(), "Hi");
    }

    #[test]
    fn binary_validation_rejects_non_01() {
        assert!(validate_input("0102", ConvType::Binary).is_err());
    }

    #[test]
    fn base64_validation_rejects_bad_chars() {
        assert!(validate_input("not!base64", ConvType::Base64).is_err());
    }

    #[test]
    fn empty_input_rejected() {
        assert!(validate_input("   ", ConvType::Text).is_err());
    }
}
