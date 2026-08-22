use gpui::{Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _, Styled, Window, div};
use gpui_component::Sizable as _;
use gpui_component::ActiveTheme as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;

use crate::app::CrittoUtil;
use crate::crypto_meta::{self, ENCRYPT_ALGORITHMS, EncryptResult};

pub fn render(app: &CrittoUtil, _window: &mut Window, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
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
                .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).child("Encrypter"))
                .child(
                    Button::new("encrypter-clear-btn")
                        .label("Clear")
                        .ghost()
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
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Text to encrypt"))
                .child(Input::new(&e.plaintext).cleanable(true)),
        )
        .child(if alg.iv_length.is_some() {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("IV (optional — leave empty to auto-generate)"))
                .child(Input::new(&e.iv).cleanable(true))
                .child(pick_from_history_row(app, "encrypter-pick-iv", cx, |this, name, window, cx| {
                    this.encrypter.iv.update(cx, |s, cx| s.set_value(name, window, cx));
                }))
                .into_any_element()
        } else {
            div().into_any_element()
        })
        .child(if alg.require_key {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Key"))
                .child(Input::new(&e.key).cleanable(true))
                .child(pick_from_history_row(app, "encrypter-pick-key", cx, |this, name, window, cx| {
                    this.encrypter.key.update(cx, |s, cx| s.set_value(name, window, cx));
                }))
                .into_any_element()
        } else {
            div().into_any_element()
        })
        .child(
            Button::new("encrypter-encrypt-btn")
                .label("Encrypt")
                .primary()
                .w_full()
                .on_click(cx.listener(|this, _, _window, cx| do_encrypt(this, cx))),
        )
        .child(if !e.error_msg.is_empty() {
            div()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().danger.opacity(0.12))
                .text_color(cx.theme().danger)
                .text_sm()
                .child(e.error_msg.clone())
                .into_any_element()
        } else {
            div().into_any_element()
        })
        .child(if !e.result_cipher.is_empty() {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().secondary)
                .child(div().text_xs().font_weight(gpui::FontWeight::BOLD).child("Ciphertext (Base64)"))
                .child(div().text_sm().child(e.result_cipher.clone()))
                .child(super::key_generator::copy_button("encrypter-copy-cipher", e.result_cipher.clone()))
                .into_any_element()
        } else {
            div().into_any_element()
        })
}

fn do_encrypt(this: &mut CrittoUtil, cx: &mut Context<CrittoUtil>) {
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

    let iv_opt = if iv.is_empty() { None } else { Some(iv.as_str()) };
    match crypto_meta::encrypt(&alg, &plaintext, &key, iv_opt) {
        Ok(EncryptResult::Plain(cipher)) => {
            this.encrypter.result_cipher = cipher;
        }
        Ok(EncryptResult::Cbc { cipher, iv: used_iv }) => {
            this.encrypter.result_cipher = cipher;
            this.encrypter.result_iv = used_iv.clone();
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
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Algorithm"))
        .child(div().flex().gap_2().flex_wrap().children(ENCRYPT_ALGORITHMS.iter().enumerate().map(|(i, alg)| {
            let selected = app.encrypter.alg_idx == i;
            let id: gpui::ElementId = format!("encrypter-alg-{i}").into();
            let btn = Button::new(id).label(alg.name);
            let btn = if selected { btn.primary() } else { btn.outline() };
            btn.on_click(cx.listener(move |this, _, window, cx| {
                this.encrypter.alg_idx = i;
                this.encrypter.plaintext.update(cx, |s, cx| s.set_value("", window, cx));
                this.encrypter.key.update(cx, |s, cx| s.set_value("", window, cx));
                this.encrypter.iv.update(cx, |s, cx| s.set_value("", window, cx));
                this.encrypter.result_cipher.clear();
                this.encrypter.result_iv.clear();
                this.encrypter.error_msg.clear();
                cx.notify();
            }))
        })))
}

fn pick_from_history_row(
    app: &CrittoUtil,
    id_prefix: &'static str,
    cx: &mut Context<CrittoUtil>,
    on_pick: impl Fn(&mut CrittoUtil, &str, &mut Window, &mut Context<CrittoUtil>) + 'static + Clone,
) -> impl IntoElement {
    if app.key_history.is_empty() {
        return div().into_any_element();
    }
    div()
        .flex()
        .gap_1()
        .flex_wrap()
        .children(app.key_history.iter().take(5).enumerate().map(|(i, entry)| {
            let name = entry.name.clone();
            let id: gpui::ElementId = format!("{id_prefix}-{i}").into();
            let on_pick = on_pick.clone();
            Button::new(id)
                .label(format!("{}… ({} bit)", &entry.name.chars().take(6).collect::<String>(), entry.bits))
                .ghost()
                .xsmall()
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_pick(this, &name, window, cx);
                }))
        }))
        .into_any_element()
}
