use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _,
    Styled, Window, div,
};
use gpui_component::ActiveTheme as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::Sizable as _;
use gpui_component::Sizable as _;
use gpui_component::input::Input;

use crate::app::CrittoUtil;
use crypto_core::crypto_meta::{self, ENCRYPT_ALGORITHMS, EncryptResult};
use crate::views::{field_with_picker, radio_row};

pub fn render(
    app: &CrittoUtil,
    _window: &mut Window,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    let e = &app.encrypter;
    let alg = app.encrypt_alg();

    div()
        .id("encrypter-view")
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .size_full()
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Encrypter"),
                )
                .child(
                    Button::new("encrypter-clear-btn")
                        .label("Clear")
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| {
                            let e = &this.encrypter;
                            e.plaintext.update(cx, |s, cx| s.set_value("", window, cx));
                            e.key.update(cx, |s, cx| s.set_value("", window, cx));
                            e.iv.update(cx, |s, cx| s.set_value("", window, cx));
                            this.encrypter.result_cipher.clear();
                            this.encrypter.result_iv.clear();
                            this.encrypter.error_msg.clear();
                            cx.notify();
                        })),
                ),
        )
        .child(alg_row(app, cx))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Text to encrypt"),
                )
                .child(Input::new(&e.plaintext).large().cleanable(true)),
        )
        .children(alg.iv_length.is_some().then(|| {
            field_with_picker(
                app,
                cx,
                "IV (optional — leave empty to auto-generate)",
                Input::new(&e.iv).large().cleanable(true),
                "encrypter-pick-iv",
                "Pick IV",
                |this, name, window, cx| {
                    this.encrypter
                        .iv
                        .update(cx, |s, cx| s.set_value(name, window, cx));
                },
            )
        }))
        .children(alg.require_key.then(|| {
            field_with_picker(
                app,
                cx,
                "Key",
                Input::new(&e.key).large().cleanable(true),
                "encrypter-pick-key",
                "Pick key",
                |this, name, window, cx| {
                    this.encrypter
                        .key
                        .update(cx, |s, cx| s.set_value(name, window, cx));
                },
            )
        }))
        .child(
            Button::new("encrypter-encrypt-btn")
                .label("Encrypt")
                .primary()
                .self_start()
                .on_click(cx.listener(|this, _, window, cx| do_encrypt(this, window, cx))),
        )
        .children((!e.error_msg.is_empty()).then(|| {
            div()
                .p_3()
                .bg(cx.theme().danger.opacity(0.12))
                .text_color(cx.theme().danger)
                .text_sm()
                .child(e.error_msg.clone())
        }))
        .children((!e.result_cipher.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .bg(cx.theme().secondary)
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Ciphertext (Base64)"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(div().text_sm().child(e.result_cipher.clone()))
                        .child(super::key_generator::copy_button(
                            "encrypter-copy-cipher",
                            e.result_cipher.clone(),
                        )),
                )
        }))
}

fn do_encrypt(this: &mut CrittoUtil, window: &mut Window, cx: &mut Context<CrittoUtil>) {
    let alg = *this.encrypt_alg();
    let plaintext = this.encrypter.plaintext.read(cx).value().to_string();
    let key = this.encrypter.key.read(cx).value().to_string();
    let iv = this.encrypter.iv.read(cx).value().to_string();

    if plaintext.is_empty() {
        this.encrypter.error_msg = "Please enter text to encrypt".to_string();
        cx.notify();
        return;
    }
    if let Some(e) = crypto_meta::key_error(&alg, &key) {
        this.encrypter.error_msg = e;
        cx.notify();
        return;
    }
    if let Some(e) = crypto_meta::iv_error(&alg, &iv) {
        this.encrypter.error_msg = e;
        cx.notify();
        return;
    }

    this.encrypter.error_msg.clear();
    this.encrypter.result_cipher.clear();
    this.encrypter.result_iv.clear();

    let iv_opt = if iv.is_empty() {
        None
    } else {
        Some(iv.as_str())
    };
    match crypto_meta::encrypt(&alg, &plaintext, &key, iv_opt) {
        Ok(EncryptResult::Plain(cipher)) => {
            this.encrypter.result_cipher = cipher;
        }
        Ok(EncryptResult::Cbc {
            cipher,
            iv: used_iv,
        }) => {
            this.encrypter.result_cipher = cipher;
            this.encrypter.result_iv = used_iv.clone();
            // Show the caller which IV was actually used, including when it
            // was auto-generated because the field was left empty.
            this.encrypter
                .iv
                .update(cx, |s, cx| s.set_value(used_iv.clone(), window, cx));
            this.add_key_history(used_iv.clone(), used_iv.len());
        }
        Err(e) => {
            this.encrypter.error_msg = e;
        }
    }
    if !key.is_empty() {
        this.add_key_history(key.clone(), key.as_bytes().len());
    }
    cx.notify();
}

fn alg_row(app: &CrittoUtil, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let labels = ENCRYPT_ALGORITHMS
        .iter()
        .map(|a| a.name.to_string())
        .collect();
    radio_row(
        app,
        cx,
        "Algorithm",
        "encrypter-alg",
        labels,
        Some(app.encrypter.alg_idx),
        |this, i, window, cx| {
            this.encrypter.alg_idx = i;
            this.encrypter
                .plaintext
                .update(cx, |s, cx| s.set_value("", window, cx));
            this.encrypter
                .key
                .update(cx, |s, cx| s.set_value("", window, cx));
            this.encrypter
                .iv
                .update(cx, |s, cx| s.set_value("", window, cx));
            this.encrypter.result_cipher.clear();
            this.encrypter.result_iv.clear();
            this.encrypter.error_msg.clear();
            cx.notify();
        },
    )
}
