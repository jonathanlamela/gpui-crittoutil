//! Agentic mode: a small tool-calling loop against a local LM Studio server
//! (OpenAI-compatible `/v1/chat/completions`). The model can call `generate_key`,
//! `encrypt`, and `decrypt`, which are executed locally against `crypto`/
//! `crypto_meta` — no network access beyond the user's own LM Studio instance.

use serde::{Deserialize, Serialize};

use crate::crypto;
use crate::crypto_meta::{self, AlgId, DECRYPT_ALGORITHMS, ENCRYPT_ALGORITHMS};

pub const DEFAULT_BASE_URL: &str = "http://localhost:1234/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: Some(content.into()), tool_calls: None, tool_call_id: None }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: Some(content.into()), tool_calls: None, tool_call_id: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    tools: &'a [ToolDef],
    temperature: f32,
}

#[derive(Serialize)]
struct ToolDef {
    #[serde(rename = "type")]
    kind: &'static str,
    function: ToolFunctionDef,
}

#[derive(Serialize)]
struct ToolFunctionDef {
    name: &'static str,
    description: &'static str,
    parameters: serde_json::Value,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

fn tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            kind: "function",
            function: ToolFunctionDef {
                name: "generate_key",
                description: "Generate a random alphanumeric key of a given bit size.",
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "bits": { "type": "integer", "enum": [64, 128, 192, 256, 512] }
                    },
                    "required": ["bits"]
                }),
            },
        },
        ToolDef {
            kind: "function",
            function: ToolFunctionDef {
                name: "encrypt",
                description: "Encrypt plaintext with MD5, AES (CBC/ECB), or DES (CBC/ECB). \
                    MD5 ignores key/iv. AES keys are 16, 24, or 32 bytes; DES keys are 8 bytes. \
                    CBC IVs match the key's block size (16 bytes for AES, 8 for DES) — leave iv \
                    empty to auto-generate one.",
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "algorithm": { "type": "string", "enum": ["MD5", "AES-CBC", "AES-ECB", "DES-ECB", "DES-CBC"] },
                        "plaintext": { "type": "string" },
                        "key": { "type": "string", "description": "Required unless algorithm is MD5." },
                        "iv": { "type": "string", "description": "Optional; only used by AES-CBC/DES-CBC." }
                    },
                    "required": ["algorithm", "plaintext"]
                }),
            },
        },
        ToolDef {
            kind: "function",
            function: ToolFunctionDef {
                name: "decrypt",
                description: "Decrypt a Base64 ciphertext with AES (CBC/ECB) or DES (CBC/ECB).",
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "algorithm": { "type": "string", "enum": ["AES-CBC", "AES-ECB", "DES-ECB", "DES-CBC"] },
                        "ciphertext": { "type": "string", "description": "Base64-encoded." },
                        "key": { "type": "string" },
                        "iv": { "type": "string", "description": "Required for CBC modes." }
                    },
                    "required": ["algorithm", "ciphertext", "key"]
                }),
            },
        },
    ]
}

fn find_alg(table: &'static [crypto_meta::AlgMeta], name: &str) -> Option<&'static crypto_meta::AlgMeta> {
    let id = match name {
        "MD5" => AlgId::Md5,
        "AES-CBC" => AlgId::AesCbc,
        "AES-ECB" => AlgId::AesEcb,
        "DES-ECB" => AlgId::DesEcb,
        "DES-CBC" => AlgId::DesCbc,
        _ => return None,
    };
    table.iter().find(|a| a.id == id)
}

/// Execute one tool call purely against `crypto`/`crypto_meta` (no app state
/// needed — algorithm/plaintext/key/iv all come from the call's own
/// arguments). Returns the JSON string to send back as the tool result, plus
/// any (name, length_bytes) key material worth remembering.
fn execute_tool(name: &str, arguments: &str) -> (String, Vec<(String, usize)>) {
    let args: serde_json::Value = match serde_json::from_str(arguments) {
        Ok(v) => v,
        Err(e) => return (format!("{{\"error\": \"invalid arguments: {e}\"}}"), Vec::new()),
    };

    match name {
        "generate_key" => {
            let bits = args.get("bits").and_then(|v| v.as_u64()).unwrap_or(256) as u32;
            match crypto::generate_key(bits) {
                Ok(key) => {
                    let entry = (key.clone(), (bits / 8) as usize);
                    (serde_json::json!({ "key": key, "bits": bits }).to_string(), vec![entry])
                }
                Err(e) => (serde_json::json!({ "error": e.message }).to_string(), Vec::new()),
            }
        }
        "encrypt" => {
            let algorithm = args.get("algorithm").and_then(|v| v.as_str()).unwrap_or("");
            let plaintext = args.get("plaintext").and_then(|v| v.as_str()).unwrap_or("");
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let iv = args.get("iv").and_then(|v| v.as_str()).filter(|s| !s.is_empty());

            let Some(alg) = find_alg(ENCRYPT_ALGORITHMS, algorithm) else {
                return (serde_json::json!({ "error": format!("unknown algorithm: {algorithm}") }).to_string(), Vec::new());
            };
            if let Some(err) = crypto_meta::key_error(alg, key) {
                return (serde_json::json!({ "error": err }).to_string(), Vec::new());
            }
            if let Some(err) = crypto_meta::iv_error(alg, iv.unwrap_or("")) {
                return (serde_json::json!({ "error": err }).to_string(), Vec::new());
            }

            match crypto_meta::encrypt(alg, plaintext, key, iv) {
                Ok(crypto_meta::EncryptResult::Plain(cipher)) => {
                    let mut new_keys = Vec::new();
                    if !key.is_empty() {
                        new_keys.push((key.to_string(), key.as_bytes().len()));
                    }
                    (serde_json::json!({ "ciphertext": cipher }).to_string(), new_keys)
                }
                Ok(crypto_meta::EncryptResult::Cbc { cipher, iv: used_iv }) => {
                    let mut new_keys = vec![(used_iv.clone(), used_iv.len())];
                    if !key.is_empty() {
                        new_keys.push((key.to_string(), key.as_bytes().len()));
                    }
                    (serde_json::json!({ "ciphertext": cipher, "iv": used_iv }).to_string(), new_keys)
                }
                Err(e) => (serde_json::json!({ "error": e }).to_string(), Vec::new()),
            }
        }
        "decrypt" => {
            let algorithm = args.get("algorithm").and_then(|v| v.as_str()).unwrap_or("");
            let ciphertext = args.get("ciphertext").and_then(|v| v.as_str()).unwrap_or("");
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let iv = args.get("iv").and_then(|v| v.as_str()).unwrap_or("");

            let Some(alg) = find_alg(DECRYPT_ALGORITHMS, algorithm) else {
                return (serde_json::json!({ "error": format!("unknown algorithm: {algorithm}") }).to_string(), Vec::new());
            };
            if let Some(err) = crypto_meta::payload_error(ciphertext) {
                return (serde_json::json!({ "error": err }).to_string(), Vec::new());
            }
            if let Some(err) = crypto_meta::iv_error_required(alg, iv) {
                return (serde_json::json!({ "error": err }).to_string(), Vec::new());
            }
            if let Some(err) = crypto_meta::key_error(alg, key) {
                return (serde_json::json!({ "error": err }).to_string(), Vec::new());
            }

            let iv_opt = if iv.trim().is_empty() { None } else { Some(iv.trim()) };
            match crypto_meta::decrypt(alg, ciphertext.trim(), key, iv_opt) {
                Ok(plaintext) => (serde_json::json!({ "plaintext": plaintext }).to_string(), Vec::new()),
                Err(e) => (serde_json::json!({ "error": e }).to_string(), Vec::new()),
            }
        }
        other => (serde_json::json!({ "error": format!("unknown tool: {other}") }).to_string(), Vec::new()),
    }
}

fn send_chat(base_url: &str, model: &str, messages: &[ChatMessage]) -> Result<ChatMessage, String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = ChatRequest { model, messages, tools: &tool_defs(), temperature: 0.2 };

    let response = ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(60))
        .send_json(&body)
        .map_err(|e| format!("Couldn't reach LM Studio at {base_url}: {e}"))?;

    let parsed: ChatResponse = response
        .into_json()
        .map_err(|e| format!("Unexpected response from LM Studio: {e}"))?;

    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message)
        .ok_or_else(|| "LM Studio returned no choices".to_string())
}

/// Run one full agent turn, starting from `history` (which already includes
/// the user's latest message) and mutating it in place with every assistant/
/// tool message produced along the way. Loops on tool calls up to a small
/// cap, then returns any keys/IVs the tools generated or used (so the caller
/// can fold them into the shared key history on the main thread) — the
/// user-visible reply is just the last assistant message in `history`.
pub fn run_turn(base_url: &str, model: &str, history: &mut Vec<ChatMessage>) -> Vec<(String, usize)> {
    let mut new_keys = Vec::new();

    for _ in 0..6 {
        let message = match send_chat(base_url, model, history) {
            Ok(m) => m,
            Err(e) => {
                history.push(ChatMessage {
                    role: "assistant".into(),
                    content: Some(e),
                    tool_calls: None,
                    tool_call_id: None,
                });
                return new_keys;
            }
        };
        history.push(message.clone());

        let Some(calls) = message.tool_calls.filter(|c| !c.is_empty()) else {
            return new_keys;
        };

        for call in calls {
            let (result, keys) = execute_tool(&call.function.name, &call.function.arguments);
            new_keys.extend(keys);
            history.push(ChatMessage {
                role: "tool".into(),
                content: Some(result),
                tool_calls: None,
                tool_call_id: Some(call.id),
            });
        }
    }

    history.push(ChatMessage {
        role: "assistant".into(),
        content: Some("Stopped after too many tool calls in a row.".to_string()),
        tool_calls: None,
        tool_call_id: None,
    });
    new_keys
}
