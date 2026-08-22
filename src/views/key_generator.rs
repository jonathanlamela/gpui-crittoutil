use gpui::{Context, InteractiveElement, IntoElement, ParentElement, Styled, Window, div};
use gpui_component::ActiveTheme as _;
use gpui_component::button::{Button, ButtonVariants as _};

use crate::app::CrittoUtil;
use crate::crypto;

const KEY_SIZES: &[u32] = &[64, 128, 192, 256, 512];

pub fn render(app: &CrittoUtil, _window: &mut Window, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let k = &app.key_generator;

    div()
        .id("key-generator-view")
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .size_full()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).child("Key generator"))
                .child(
                    Button::new("keygen-clear-btn")
                        .label("Clear")
                        .ghost()
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.key_generator.generated_key.clear();
                            cx.notify();
                        })),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Key size"))
                .child(div().flex().gap_2().flex_wrap().children(KEY_SIZES.iter().map(|&bits| {
                    let selected = k.key_size == bits;
                    let id: gpui::ElementId = format!("keygen-size-{bits}").into();
                    let btn = Button::new(id).label(format!("{bits} bit"));
                    let btn = if selected { btn.primary() } else { btn.outline() };
                    btn.on_click(cx.listener(move |this, _, _window, cx| {
                        this.key_generator.key_size = bits;
                        cx.notify();
                    }))
                }))),
        )
        .child(
            Button::new("keygen-generate-btn")
                .label("Generate key")
                .primary()
                .w_full()
                .on_click(cx.listener(|this, _, _window, cx| {
                    let bits = this.key_generator.key_size;
                    match crypto::generate_key(bits) {
                        Ok(key) => {
                            this.key_generator.generated_key = key.clone();
                            this.add_key_history(key, (bits / 8) as usize);
                        }
                        Err(e) => {
                            this.key_generator.generated_key = format!("Error: {}", e.message);
                        }
                    }
                    cx.notify();
                })),
        )
        .child(if !k.generated_key.is_empty() {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().secondary)
                .child(div().text_xs().font_weight(gpui::FontWeight::BOLD).child("Generated key"))
                .child(div().text_sm().child(format!("{} ({} bit)", k.generated_key, k.key_size)))
                .child(copy_button("keygen-copy-btn", k.generated_key.clone()))
                .into_any_element()
        } else {
            div().into_any_element()
        })
        .child(key_history_list(app, cx))
}

pub fn copy_button(id: &'static str, value: String) -> impl IntoElement {
    Button::new(id).label("Copy").ghost().on_click(move |_, _window, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.clone()));
    })
}

pub fn key_history_list(app: &CrittoUtil, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    if app.key_history.is_empty() {
        return div().into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_xs().font_weight(gpui::FontWeight::BOLD).child("History"))
        .children(app.key_history.iter().enumerate().map(|(i, entry)| {
            let value = entry.name.clone();
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .p_2()
                .rounded(cx.theme().radius)
                .bg(cx.theme().secondary)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().text_sm().child(entry.name.clone()))
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(format!("{} bit", entry.bits))),
                )
                .child(copy_button_dyn(format!("keygen-hist-copy-{i}"), value))
        }))
        .into_any_element()
}

fn copy_button_dyn(id: String, value: String) -> impl IntoElement {
    Button::new(gpui::ElementId::from(id)).label("Copy").ghost().on_click(move |_, _window, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.clone()));
    })
}
