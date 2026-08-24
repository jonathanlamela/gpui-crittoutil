use gpui::{
    AppContext as _, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use gpui_component::ActiveTheme as _;
use gpui_component::IconName;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::InputState;

use crate::agent::{self, ChatMessage};
use crate::converter::ConvType;
use crate::crypto_meta::{AlgId, DECRYPT_ALGORITHMS, ENCRYPT_ALGORITHMS};
use crate::session::{self, Session, StoredKeyEntry};
use crate::views;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Home,
    Converter,
    KeyGenerator,
    Encrypter,
    Decrypter,
    FileHasher,
}

impl Route {
    pub const ALL: [Route; 6] = [
        Route::Home,
        Route::Converter,
        Route::KeyGenerator,
        Route::Encrypter,
        Route::Decrypter,
        Route::FileHasher,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Route::Home => "Home",
            Route::Converter => "Converter",
            Route::KeyGenerator => "Key generator",
            Route::Encrypter => "Encrypter",
            Route::Decrypter => "Decrypter",
            Route::FileHasher => "File hasher",
        }
    }
}

/// A single entry in the shared key-history list. Mirrors the Pinia `keyHistory`
/// store: newest first, no duplicate names.
#[derive(Debug, Clone)]
pub struct KeyEntry {
    pub name: String,
    pub bits: usize,
}

pub struct ConverterState {
    pub input: Entity<InputState>,
    pub from_type: ConvType,
    pub to_type: ConvType,
    pub output: String,
    pub input_error: String,
}

pub struct KeyGeneratorState {
    pub key_size: u32,
    pub generated_key: String,
    /// The bit size the currently-displayed `generated_key` was actually
    /// generated at — independent of `key_size`, which tracks the radio
    /// selection and can change afterwards without regenerating the key.
    pub generated_bits: u32,
}

pub struct EncrypterState {
    pub alg_idx: usize,
    pub plaintext: Entity<InputState>,
    pub key: Entity<InputState>,
    pub iv: Entity<InputState>,
    pub result_cipher: String,
    pub result_iv: String,
    pub error_msg: String,
}

pub struct DecrypterState {
    pub alg_idx: usize,
    pub payload: Entity<InputState>,
    pub key: Entity<InputState>,
    pub iv: Entity<InputState>,
    pub result: String,
    pub error_msg: String,
}

pub struct FileHasherState {
    pub filename: String,
    pub filesize: u64,
    pub hash: String,
}

/// State for agentic mode: a chat with a local LM Studio model that can call
/// the app's own crypto tools (generate/encrypt/decrypt/convert) on the
/// user's behalf. Always talks to `agent::DEFAULT_BASE_URL`, auto-detecting
/// whichever model LM Studio has loaded — no endpoint/model picker in the UI.
/// The conversation lives on this always-alive entity, so it survives
/// closing and reopening the panel; it only resets when the app restarts.
pub struct AgentState {
    pub open: bool,
    pub messages: Vec<ChatMessage>,
    pub input: Entity<InputState>,
    pub is_running: bool,
    /// Indices into `messages` of assistant turns whose tool-call group is
    /// currently expanded in the chat panel (collapsed by default).
    pub expanded_tool_calls: std::collections::HashSet<usize>,
}

/// The single top-level entity for the whole app. Per this project's house style
/// (see gpui-playground/CLAUDE.md's entity-nesting gotcha), ALL view state lives
/// here as plain fields; everything else is rendered via plain functions, never
/// via additional `cx.new(...)` entities. `Entity<InputState>` fields are the only
/// exception — they are gpui-component's own leaf state entities for text editing
/// and are required for text input to work at all.
pub struct CrittoUtil {
    pub route: Route,
    pub key_history: Vec<KeyEntry>,

    /// Saved sessions, newest first — loaded from disk at startup. `None`
    /// active session means the user is at the session picker, not inside
    /// the 6-screen app.
    pub sessions: Vec<Session>,
    pub active_session_id: Option<String>,

    pub home_search: Entity<InputState>,

    pub converter: ConverterState,
    pub key_generator: KeyGeneratorState,
    pub encrypter: EncrypterState,
    pub decrypter: DecrypterState,
    pub file_hasher: FileHasherState,
    pub agent: AgentState,
}

impl CrittoUtil {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let agent_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Ask the agent to generate a key, encrypt, or decrypt…")
        });
        cx.subscribe_in(&agent_input, window, |this, _input, event, window, cx| {
            if let gpui_component::input::InputEvent::PressEnter { .. } = event {
                this.send_agent_message(window, cx);
            }
        })
        .detach();

        Self {
            route: Route::Home,
            key_history: Vec::new(),
            sessions: session::load_all(),
            active_session_id: None,
            home_search: cx
                .new(|cx| InputState::new(window, cx).placeholder("What do you want to do?")),
            converter: ConverterState {
                input: cx.new(|cx| {
                    InputState::new(window, cx).placeholder("Enter a value to convert...")
                }),
                from_type: ConvType::Text,
                to_type: ConvType::Base64,
                output: String::new(),
                input_error: String::new(),
            },
            key_generator: KeyGeneratorState {
                key_size: 256,
                generated_key: String::new(),
                generated_bits: 0,
            },
            encrypter: EncrypterState {
                alg_idx: 0,
                plaintext: cx.new(|cx| InputState::new(window, cx).placeholder("Text to encrypt")),
                key: cx.new(|cx| InputState::new(window, cx).placeholder("Encryption key")),
                iv: cx.new(|cx| {
                    InputState::new(window, cx).placeholder("IV (leave empty to auto-generate)")
                }),
                result_cipher: String::new(),
                result_iv: String::new(),
                error_msg: String::new(),
            },
            decrypter: DecrypterState {
                alg_idx: 0,
                payload: cx.new(|cx| InputState::new(window, cx).placeholder("Base64 ciphertext")),
                key: cx.new(|cx| InputState::new(window, cx).placeholder("Decryption key")),
                iv: cx.new(|cx| InputState::new(window, cx).placeholder("IV")),
                result: String::new(),
                error_msg: String::new(),
            },
            file_hasher: FileHasherState {
                filename: String::new(),
                filesize: 0,
                hash: String::new(),
            },
            agent: AgentState {
                open: false,
                messages: Vec::new(),
                input: agent_input,
                is_running: false,
                expanded_tool_calls: std::collections::HashSet::new(),
            },
        }
    }

    /// Add a key/IV to the shared history: newest first, no duplicates by name.
    /// Mirrors the Pinia `keyHistory` store's `addKey(name, lengthBytes)`.
    pub fn add_key_history(&mut self, name: String, length_bytes: usize) {
        if name.is_empty() || self.key_history.iter().any(|k| k.name == name) {
            return;
        }
        self.key_history.insert(
            0,
            KeyEntry {
                name,
                bits: length_bytes * 8,
            },
        );
        self.persist_active_session();
    }

    pub fn navigate(&mut self, route: Route, cx: &mut Context<Self>) {
        self.route = route;
        cx.notify();
    }

    /// Create a brand-new session, make it active, and enter the app on Home.
    pub fn create_session(&mut self, cx: &mut Context<Self>) {
        let session = session::new_session(self.sessions.len());
        self.active_session_id = Some(session.id.clone());
        self.sessions.insert(0, session);
        self.key_history.clear();
        self.route = Route::Home;
        session::save_all(&self.sessions);
        cx.notify();
    }

    /// Open a previously-saved session: restore its key history and enter
    /// the app on Home.
    pub fn open_session(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.iter().find(|s| s.id == id) else {
            return;
        };
        self.key_history = session
            .key_history
            .iter()
            .cloned()
            .map(KeyEntry::from)
            .collect();
        self.active_session_id = Some(id.to_string());
        self.route = Route::Home;
        cx.notify();
    }

    /// Leave the active session and return to the session picker.
    pub fn close_session(&mut self, cx: &mut Context<Self>) {
        self.persist_active_session();
        self.active_session_id = None;
        self.key_history.clear();
        cx.notify();
    }

    /// Write the current `key_history` back into the active session's record
    /// and save the whole session list to disk. No-op if no session is
    /// active (e.g. still on the picker).
    fn persist_active_session(&mut self) {
        let Some(active_id) = self.active_session_id.clone() else {
            return;
        };
        if let Some(session) = self.sessions.iter_mut().find(|s| s.id == active_id) {
            session.key_history = self.key_history.iter().map(StoredKeyEntry::from).collect();
        }
        session::save_all(&self.sessions);
    }

    pub fn toggle_agent(&mut self, cx: &mut Context<Self>) {
        self.agent.open = !self.agent.open;
        cx.notify();
    }

    /// Send the agent panel's current input as a user message, then run the
    /// tool-calling loop against LM Studio in the background and fold the
    /// result (reply + any generated/used keys) back in once it completes.
    pub fn send_agent_message(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.agent.input.read(cx).value().trim().to_string();
        if text.is_empty() || self.agent.is_running {
            return;
        }
        self.agent
            .input
            .update(cx, |s, cx| s.set_value("", window, cx));
        if self.agent.messages.is_empty() {
            self.agent.messages.push(ChatMessage::system(
                "You are a helpful assistant embedded in a crypto utility app. Always reply in \
                 English, regardless of what language the user writes in. You cannot generate \
                 random keys, encrypt, decrypt, or convert between text/binary/Base64 by \
                 yourself — you have no access to randomness, cipher implementations, or \
                 encoding logic. When the user asks for a concrete action (encrypt/decrypt \
                 something, generate a key, convert/encode/decode a value), you MUST call the \
                 appropriate tool (generate_key, encrypt, decrypt, or convert) in that same turn \
                 using the function/tool-calling mechanism the API gives you. Never write a \
                 tool name as plain text, e.g. `encrypt(\"hello\")` or a JSON blob describing the \
                 call — that does nothing at all, it is not a real call, and never invent, \
                 guess, or compute a key/ciphertext/plaintext/encoded value yourself. A tool's \
                 result is already the final answer — report the exact value it returned.",
            ));
        }
        self.agent.messages.push(ChatMessage::user(text));
        self.agent.is_running = true;
        cx.notify();

        let mut history = self.agent.messages.clone();

        cx.spawn(async move |entity, cx| {
            let (history, new_keys) = cx
                .background_executor()
                .spawn(async move {
                    let base_url = agent::DEFAULT_BASE_URL;
                    // Always ask LM Studio which model it actually has loaded
                    // rather than requiring the user to type it in.
                    let model = agent::fetch_first_model_id(base_url).unwrap_or_else(|| "local-model".to_string());
                    let new_keys = agent::run_turn(base_url, &model, &mut history);
                    (history, new_keys)
                })
                .await;

            entity
                .update(cx, |this, cx| {
                    this.agent.messages = history;
                    for (name, bytes) in new_keys {
                        this.add_key_history(name, bytes);
                    }
                    this.agent.is_running = false;
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    /// Expand/collapse a tool-call group in the agent chat panel — indexed by
    /// the position of the assistant message that starts it.
    pub fn toggle_tool_group(&mut self, index: usize, cx: &mut Context<Self>) {
        if !self.agent.expanded_tool_calls.remove(&index) {
            self.agent.expanded_tool_calls.insert(index);
        }
        cx.notify();
    }

    pub fn encrypt_alg(&self) -> &'static crate::crypto_meta::AlgMeta {
        &ENCRYPT_ALGORITHMS[self.encrypter.alg_idx]
    }

    pub fn decrypt_alg(&self) -> &'static crate::crypto_meta::AlgMeta {
        &DECRYPT_ALGORITHMS[self.decrypter.alg_idx]
    }
}

impl Render for CrittoUtil {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The Root wrapping this view does not render dialogs/sheets/notifications
        // itself in the pinned gpui-component revision — the top-level view has to
        // pull those layers in explicitly, or `window.open_dialog(...)` etc. push
        // state that never gets painted.
        let sheet_layer = gpui_component::Root::render_sheet_layer(window, cx);
        let dialog_layer = gpui_component::Root::render_dialog_layer(window, cx);
        let notification_layer = gpui_component::Root::render_notification_layer(window, cx);

        div()
            .id("crittoutil-root")
            .size_full()
            .child(if self.active_session_id.is_none() {
                views::session_picker::render(self, window, cx).into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .bg(cx.theme().background)
                    .text_color(cx.theme().foreground)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_1()
                            .min_h_0()
                            .child(views::sidebar::render(self, window, cx))
                            .child(
                                div()
                                    .id("crittoutil-content")
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .h_full()
                                    .overflow_hidden()
                                    .mt_4()
                                    // Fixed container, common to every screen — currently just
                                    // the agent toggle, but any chrome that should appear above
                                    // every section (not just some) belongs here, not repeated
                                    // per-view.
                                    .child(
                                        div()
                                            .flex()
                                            .justify_end()
                                            // Matches each screen's own `.p_6()` inset below, so
                                            // this button's right edge lines up with theirs.
                                            .px_6()
                                            .pt_3()
                                            .child(
                                                Button::new("toggle-agent-mode")
                                                    .icon(IconName::Bot)
                                                    .label("Agent")
                                                    .map(|btn| {
                                                        if self.agent.open { btn.primary() } else { btn.info() }
                                                    })
                                                    .on_click(cx.listener(|this, _, _window, cx| {
                                                        this.toggle_agent(cx);
                                                    })),
                                            ),
                                    )
                                    // The section-specific container for whichever screen is active.
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_h_0()
                                            .overflow_hidden()
                                            .child(match self.route {
                                                Route::Home => {
                                                    views::home::render(self, window, cx).into_any_element()
                                                }
                                                Route::Converter => {
                                                    views::converter::render(self, window, cx)
                                                        .into_any_element()
                                                }
                                                Route::KeyGenerator => {
                                                    views::key_generator::render(self, window, cx)
                                                        .into_any_element()
                                                }
                                                Route::Encrypter => {
                                                    views::encrypter::render(self, window, cx)
                                                        .into_any_element()
                                                }
                                                Route::Decrypter => {
                                                    views::decrypter::render(self, window, cx)
                                                        .into_any_element()
                                                }
                                                Route::FileHasher => {
                                                    views::file_hasher::render(self, window, cx)
                                                        .into_any_element()
                                                }
                                            }),
                                    ),
                            )
                            .when(self.agent.open, |row| {
                                row.child(views::agent_panel::render(self, window, cx))
                            }),
                    )
                    .into_any_element()
            })
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}

#[allow(unused)]
pub(crate) fn alg_id_label(id: AlgId) -> &'static str {
    match id {
        AlgId::Md5 => "MD5",
        AlgId::AesCbc => "AES (CBC)",
        AlgId::AesEcb => "AES (ECB)",
        AlgId::DesEcb => "DES (ECB)",
        AlgId::DesCbc => "DES (CBC)",
    }
}
