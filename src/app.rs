use gpui::{AppContext as _, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window, div};
use gpui_component::ActiveTheme as _;
use gpui_component::input::InputState;

use crate::converter::ConvType;
use crate::crypto_meta::{AlgId, DECRYPT_ALGORITHMS, ENCRYPT_ALGORITHMS};
use crate::ui::style;
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

/// The single top-level entity for the whole app. Per this project's house style
/// (see gpui-playground/CLAUDE.md's entity-nesting gotcha), ALL view state lives
/// here as plain fields; everything else is rendered via plain functions, never
/// via additional `cx.new(...)` entities. `Entity<InputState>` fields are the only
/// exception — they are gpui-component's own leaf state entities for text editing
/// and are required for text input to work at all.
pub struct CrittoUtil {
    pub route: Route,
    pub key_history: Vec<KeyEntry>,

    pub home_search: Entity<InputState>,

    pub converter: ConverterState,
    pub key_generator: KeyGeneratorState,
    pub encrypter: EncrypterState,
    pub decrypter: DecrypterState,
    pub file_hasher: FileHasherState,
}

impl CrittoUtil {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            route: Route::Home,
            key_history: Vec::new(),
            home_search: cx.new(|cx| {
                InputState::new(window, cx).placeholder("What do you want to do?")
            }),
            converter: ConverterState {
                input: cx.new(|cx| InputState::new(window, cx).placeholder("Enter a value to convert...")),
                from_type: ConvType::Text,
                to_type: ConvType::Base64,
                output: String::new(),
                input_error: String::new(),
            },
            key_generator: KeyGeneratorState {
                key_size: 256,
                generated_key: String::new(),
            },
            encrypter: EncrypterState {
                alg_idx: 0,
                plaintext: cx.new(|cx| InputState::new(window, cx).placeholder("Text to encrypt")),
                key: cx.new(|cx| InputState::new(window, cx).placeholder("Encryption key")),
                iv: cx.new(|cx| InputState::new(window, cx).placeholder("IV (leave empty to auto-generate)")),
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
        }
    }

    /// Add a key/IV to the shared history: newest first, no duplicates by name.
    /// Mirrors the Pinia `keyHistory` store's `addKey(name, lengthBytes)`.
    pub fn add_key_history(&mut self, name: String, length_bytes: usize) {
        if name.is_empty() || self.key_history.iter().any(|k| k.name == name) {
            return;
        }
        self.key_history.insert(0, KeyEntry { name, bits: length_bytes * 8 });
    }

    pub fn navigate(&mut self, route: Route, cx: &mut Context<Self>) {
        self.route = route;
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
        div()
            .id("crittoutil-root")
            .flex()
            .flex_row()
            .gap_3()
            .p_3()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(views::sidebar::render(self, window, cx))
            .child(
                style::card(
                    div()
                        .id("crittoutil-content")
                        .flex_1()
                        .h_full()
                        .p_5()
                        .overflow_hidden(),
                    cx,
                )
                .child(match self.route {
                        Route::Home => views::home::render(self, window, cx).into_any_element(),
                        Route::Converter => views::converter::render(self, window, cx).into_any_element(),
                        Route::KeyGenerator => views::key_generator::render(self, window, cx).into_any_element(),
                        Route::Encrypter => views::encrypter::render(self, window, cx).into_any_element(),
                        Route::Decrypter => views::decrypter::render(self, window, cx).into_any_element(),
                        Route::FileHasher => views::file_hasher::render(self, window, cx).into_any_element(),
                    }),
            )
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
