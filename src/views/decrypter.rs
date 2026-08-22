use gpui::{Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _, Styled, Window, div};
use gpui_component::Sizable as _;
use gpui_component::ActiveTheme as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;

use crate::app::CrittoUtil;
use crate::crypto_meta::{self, DECRYPT_ALGORITHMS};

pub fn render(app: &CrittoUtil, _window: &mut Window, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let d = &app.decrypter;
    let alg = app.decrypt_alg();

    div()
        .id("decrypter-view")
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
                .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).child("Decrypter"))
                .child(
                    Button::new("decrypter-clear-btn")
                        .label("Clear")
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| {
                            let d = &this.decrypter;
                            d.payload.update(cx, |s, cx| s.set_value("", window, cx));
                            d.key.update(cx, |s, cx| s.set_value("", window, cx));
                            d.iv.update(cx, |s, cx| s.set_value("", window, cx));
                            this.decrypter.result.clear();
                            this.decrypter.error_msg.clear();
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
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Base64 payload to decrypt"))
                .child(Input::new(&d.payload).cleanable(true)),
        )
        .child(if alg.iv_length.is_some() {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("IV"))
                .child(Input::new(&d.iv).cleanable(true))
                .child(pick_from_history_row(app, "decrypter-pick-iv", cx, |this, name, window, cx| {
                    this.decrypter.iv.update(cx, |s, cx| s.set_value(name, window, cx));
                }))
                .into_any_element()
        } else {
            div().into_any_element()
        })
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Key"))
                .child(Input::new(&d.key).cleanable(true))
                .child(pick_from_history_row(app, "decrypter-pick-key", cx, |this, name, window, cx| {
                    this.decrypter.key.update(cx, |s, cx| s.set_value(name, window, cx));
                })),
        )
        .child(
            Button::new("decrypter-decrypt-btn")
                .label("Decrypt")
                .primary()
                .w_full()
                .on_click(cx.listener(|this, _, _window, cx| do_decrypt(this, cx))),
        )
        .child(if !d.error_msg.is_empty() {
            div()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().danger.opacity(0.12))
                .text_color(cx.theme().danger)
                .text_sm()
                .child(d.error_msg.clone())
                .into_any_element()
        } else {
            div().into_any_element()
        })
        .child(if !d.result.is_empty() {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().secondary)
                .child(div().text_xs().font_weight(gpui::FontWeight::BOLD).child("Decrypted text"))
                .child(div().text_sm().child(d.result.clone()))
                .child(super::key_generator::copy_button("decrypter-copy-result", d.result.clone()))
                .into_any_element()
        } else {
            div().into_any_element()
        })
}

fn do_decrypt(this: &mut CrittoUtil, cx: &mut Context<CrittoUtil>) {
    let alg = *this.decrypt_alg();
    let payload = this.decrypter.payload.read(cx).value().to_string();
    let key = this.decrypter.key.read(cx).value().to_string();
    let iv = this.decrypter.iv.read(cx).value().to_string();

    if let Some(e) = crypto_meta::payload_error(&payload) {
        this.decrypter.error_msg = e;
        cx.notify();
        return;
    }
    if let Some(e) = crypto_meta::iv_error_required(&alg, &iv) {
        this.decrypter.error_msg = e;
        cx.notify();
        return;
    }
    if let Some(e) = crypto_meta::key_error(&alg, &key) {
        this.decrypter.error_msg = e;
        cx.notify();
        return;
    }

    this.decrypter.error_msg.clear();
    this.decrypter.result.clear();

    let iv_opt = if iv.trim().is_empty() { None } else { Some(iv.trim()) };
    match crypto_meta::decrypt(&alg, payload.trim(), &key, iv_opt) {
        Ok(result) => {
            this.decrypter.result = result;
            this.add_key_history(key.clone(), key.as_bytes().len());
            if let Some(iv_trimmed) = iv_opt {
                this.add_key_history(iv_trimmed.to_string(), iv_trimmed.len());
            }
        }
        Err(e) => {
            this.decrypter.error_msg = e;
        }
    }
    cx.notify();
}

fn alg_row(app: &CrittoUtil, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Algorithm"))
        .child(div().flex().gap_2().flex_wrap().children(DECRYPT_ALGORITHMS.iter().enumerate().map(|(i, alg)| {
            let selected = app.decrypter.alg_idx == i;
            let id: gpui::ElementId = format!("decrypter-alg-{i}").into();
            let btn = Button::new(id).label(alg.name);
            let btn = if selected { btn.primary() } else { btn.outline() };
            btn.on_click(cx.listener(move |this, _, window, cx| {
                this.decrypter.alg_idx = i;
                this.decrypter.payload.update(cx, |s, cx| s.set_value("", window, cx));
                this.decrypter.key.update(cx, |s, cx| s.set_value("", window, cx));
                this.decrypter.iv.update(cx, |s, cx| s.set_value("", window, cx));
                this.decrypter.result.clear();
                this.decrypter.error_msg.clear();
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
