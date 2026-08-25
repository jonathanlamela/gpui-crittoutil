//! Agentic mode: a small tool-calling loop against a local LM Studio server
//! (OpenAI-compatible `/v1/chat/completions`). The model can call `generate_key`,
//! `encrypt`, `decrypt`, and `convert`, which are executed locally against
//! `crypto`/`crypto_meta`/`converter` — no network access beyond the user's
//! own LM Studio instance.

use serde::{Deserialize, Serialize};

use converter::{self, ConvType};
use crypto_core::crypto;
use crypto_core::crypto_meta::{self, AlgId, DECRYPT_ALGORITHMS, ENCRYPT_ALGORITHMS};

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
    /// UI-only: for a `role: "tool"` message, which tool produced this result
    /// and the arguments it was called with (`content` is the result itself).
    /// Never sent to the API — just enough for the chat panel to render a
    /// collapsible tool-call block, the same way whether the call came from
    /// a real `tool_calls` entry or the text-narrated fallback (which has no
    /// structured call of its own to read the name/arguments back from).
    #[serde(skip)]
    pub display: Option<(String, String)>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: Some(content.into()), tool_calls: None, tool_call_id: None, display: None }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: Some(content.into()), tool_calls: None, tool_call_id: None, display: None }
    }

    fn tool_result(tool_call_id: String, name: String, arguments: String, result: String) -> Self {
        Self {
            role: "tool".into(),
            content: Some(result),
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
            display: Some((name, arguments)),
        }
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
                name: "convert",
                description: "Convert a value between plain text, binary (space-separated \
                    8-bit groups), and Base64. Not encryption — use this for encoding/decoding \
                    requests like \"convert X to base64\" or \"decode this base64 string\".",
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "from": { "type": "string", "enum": ["text", "binary", "base64"] },
                        "to": { "type": "string", "enum": ["text", "binary", "base64"] },
                        "value": { "type": "string" }
                    },
                    "required": ["from", "to", "value"]
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

const TOOL_NAMES: &[&str] = &["generate_key", "encrypt", "decrypt", "convert"];

fn find_conv_type(name: &str) -> Option<ConvType> {
    match name {
        "text" => Some(ConvType::Text),
        "binary" => Some(ConvType::Binary),
        "base64" => Some(ConvType::Base64),
        _ => None,
    }
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
/// Scan free-form text for the first balanced `{ ... }` object that parses as
/// a tool call — the shape a model writes when it "calls" a tool as plain
/// text instead of a real `tool_calls` entry. Accepts the same field aliases
/// models commonly use (`name`/`tool`/`function`, `arguments`/`parameters`/
/// `args`) and only matches a name from `TOOL_NAMES`, so unrelated JSON in
/// the reply can't be mistaken for a call. Returns the tool name and its
/// arguments re-serialized as a JSON string (matching what `execute_tool`
/// expects from a real call).
fn extract_pseudo_tool_call(content: &str) -> Option<(String, String)> {
    for (start, _) in content.match_indices('{') {
        let mut depth = 0i32;
        for (offset, ch) in content[start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let end = start + offset + ch.len_utf8();
                        let candidate = &content[start..end];
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(candidate) {
                            let name = value
                                .get("name")
                                .or_else(|| value.get("tool"))
                                .or_else(|| value.get("function"))
                                .and_then(|v| v.as_str());
                            let arguments = value
                                .get("arguments")
                                .or_else(|| value.get("parameters"))
                                .or_else(|| value.get("args"));
                            if let (Some(name), Some(arguments)) = (name, arguments) {
                                if TOOL_NAMES.contains(&name) && arguments.is_object() {
                                    return Some((name.to_string(), arguments.to_string()));
                                }
                            }
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Detect a bare, non-JSON "call" like `encrypt("hello")` or `generate_key
/// 256` — text that names a real tool but carries no parseable arguments, so
/// there's nothing to execute. The caller should ask the model to redo it as
/// a real tool call rather than silently giving up.
fn looks_like_bare_tool_call(content: &str) -> bool {
    let trimmed = content.trim_start();
    let name_end = trimmed
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(trimmed.len());
    if name_end == 0 {
        return false;
    }
    let name = &trimmed[..name_end];
    if !TOOL_NAMES.contains(&name) {
        return false;
    }
    match trimmed[name_end..].chars().next() {
        Some(c) => c == '(' || c == '"' || c.is_whitespace(),
        None => false,
    }
}

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
        "convert" => {
            let from = args.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let to = args.get("to").and_then(|v| v.as_str()).unwrap_or("");
            let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");

            let (Some(from_type), Some(to_type)) = (find_conv_type(from), find_conv_type(to)) else {
                return (
                    serde_json::json!({ "error": format!("unknown conversion type: {from} or {to}") }).to_string(),
                    Vec::new(),
                );
            };
            if let Err(err) = converter::validate_input(value, from_type) {
                return (serde_json::json!({ "error": err }).to_string(), Vec::new());
            }
            match converter::convert(value, from_type, to_type) {
                Ok(result) => (serde_json::json!({ "result": result }).to_string(), Vec::new()),
                Err(e) => (serde_json::json!({ "error": e }).to_string(), Vec::new()),
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

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    id: String,
}

/// Ask LM Studio which model it currently has loaded (`GET /models`) and use
/// the first one — mirrors what a user would otherwise have to type in by
/// hand, and sidesteps a mismatched/empty model id causing a rejected or
/// misrouted request.
pub fn fetch_first_model_id(base_url: &str) -> Option<String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .ok()?;
    let parsed: ModelsResponse = response.into_json().ok()?;
    parsed.data.into_iter().next().map(|m| m.id)
}

fn send_chat(base_url: &str, model: &str, messages: &[ChatMessage]) -> Result<ChatMessage, String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = ChatRequest {
        model,
        messages,
        tools: &tool_defs(),
        temperature: 0.2,
    };

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

    for round in 0..6 {
        let message = match send_chat(base_url, model, history) {
            Ok(m) => m,
            Err(e) => {
                history.push(ChatMessage {
                    role: "assistant".into(),
                    content: Some(e),
                    tool_calls: None,
                    tool_call_id: None,
                    display: None,
                });
                return new_keys;
            }
        };
        history.push(message.clone());

        let Some(calls) = message.tool_calls.filter(|c| !c.is_empty()) else {
            let content = message.content.as_deref().unwrap_or("");

            // Some local models/servers never populate `tool_calls` and instead
            // write a function-call-shaped JSON blob straight into their text
            // reply. If we can find one, execute it for real instead of
            // trusting whatever fake "result" the model made up right after it.
            if let Some((name, arguments)) = extract_pseudo_tool_call(content) {
                let (result, keys) = execute_tool(&name, &arguments);
                new_keys.extend(keys);
                // The assistant's own text was just a narrated (and possibly
                // fabricated) attempt at calling the tool — not something
                // worth showing as its own chat bubble now that we're about
                // to show the real call/result as a tool-call group.
                if let Some(last) = history.last_mut() {
                    last.content = None;
                }
                history.push(ChatMessage::tool_result(
                    format!("fallback-{round}"),
                    name,
                    arguments,
                    result,
                ));
                continue;
            }

            // A bare, argument-less mention like `encrypt("hello")` names a
            // real tool but carries nothing we can execute — push it back on
            // the model instead of giving up.
            if looks_like_bare_tool_call(content) {
                history.push(ChatMessage::user(
                    "That was not a real tool call, just text — nothing happened. Invoke the \
                     tool for real using the function/tool-calling mechanism, with all its \
                     required arguments.",
                ));
                continue;
            }

            return new_keys;
        };

        for call in calls {
            let (result, keys) = execute_tool(&call.function.name, &call.function.arguments);
            new_keys.extend(keys);
            history.push(ChatMessage::tool_result(
                call.id,
                call.function.name,
                call.function.arguments,
                result,
            ));
        }
    }

    history.push(ChatMessage {
        role: "assistant".into(),
        content: Some("Stopped after too many tool calls in a row.".to_string()),
        tool_calls: None,
        tool_call_id: None,
        display: None,
    });
    new_keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_pseudo_tool_call_from_fenced_and_bare_json() {
        // Real qwen2.5-coder-7b-instruct output: it never populates `tool_calls`
        // and instead writes the call as text, once inside a ```json fence and
        // once bare — the parser must grab the first well-formed one.
        let content = "To convert \"ciao mondo\" to Base64, I'll use the following function call:\n\n\
            ```json\n{\n  \"name\": \"encrypt\",\n  \"arguments\": {\n    \"algorithm\": \"MD5\",\n    \"plaintext\": \"ciao mondo\"\n  }\n}\n```\n\n\
            The result of this operation is:\n{\"name\":\"encrypt\",\"arguments\":{\"algorithm\":\"MD5\",\"plaintext\":\"ciao mondo\"}}";

        let (name, arguments) = extract_pseudo_tool_call(content).expect("should find a pseudo tool call");
        assert_eq!(name, "encrypt");
        let args: serde_json::Value = serde_json::from_str(&arguments).unwrap();
        assert_eq!(args["algorithm"], "MD5");
        assert_eq!(args["plaintext"], "ciao mondo");
    }

    #[test]
    fn no_pseudo_tool_call_in_plain_text() {
        assert!(extract_pseudo_tool_call("Sure, here's the answer: 42.").is_none());
    }

    #[test]
    fn extracts_pseudo_tool_call_with_field_aliases() {
        let content = r#"{"tool": "generate_key", "parameters": {"bits": 128}}"#;
        let (name, arguments) = extract_pseudo_tool_call(content).unwrap();
        assert_eq!(name, "generate_key");
        let args: serde_json::Value = serde_json::from_str(&arguments).unwrap();
        assert_eq!(args["bits"], 128);
    }

    #[test]
    fn ignores_json_naming_an_unknown_tool() {
        // Must not fire on arbitrary JSON that happens to have name/arguments
        // keys but doesn't name one of our real tools.
        let content = r#"{"name": "delete_everything", "arguments": {}}"#;
        assert!(extract_pseudo_tool_call(content).is_none());
    }

    #[test]
    fn detects_bare_tool_call_without_json() {
        assert!(looks_like_bare_tool_call("encrypt(\"hello\")"));
        assert!(looks_like_bare_tool_call("generate_key 256"));
        assert!(!looks_like_bare_tool_call("Sure, I can help with that."));
        assert!(!looks_like_bare_tool_call("encryption is important"));
    }

    #[test]
    fn convert_tool_encodes_text_to_base64() {
        let (result, keys) = execute_tool("convert", r#"{"from":"text","to":"base64","value":"ciao mondo"}"#);
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["result"], "Y2lhbyBtb25kbw==");
        assert!(keys.is_empty());
    }
}
