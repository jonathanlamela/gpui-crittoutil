use gpui::{
    Context, Corners, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement as _, Styled, Window, div, px,
};
use gpui_component::ActiveTheme as _;
use gpui_component::IconName;
use gpui_component::Sizable as _;
use gpui_component::StyledExt;
use gpui_component::button::{Button, ButtonVariants as _};

use crate::app::CrittoUtil;
use crate::crypto;
use crate::views::{radio_row, result_tile};

const KEY_SIZES: &[u32] = &[64, 128, 192, 256, 512];

pub fn render(
    app: &CrittoUtil,
    _window: &mut Window,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    let k = &app.key_generator;

    div()
        .id("key-generator-view")
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
                        .child("Key generator"),
                )
                .child(
                    Button::new("keygen-clear-btn")
                        .label("Clear")
                        .outline()
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.key_generator.generated_key.clear();
                            cx.notify();
                        })),
                ),
        )
        .child({
            let labels = KEY_SIZES.iter().map(|bits| format!("{bits} bit")).collect();
            let selected_index = KEY_SIZES.iter().position(|&bits| bits == k.key_size);
            radio_row(
                app,
                cx,
                "Key size",
                "keygen-size",
                labels,
                selected_index,
                |this, i, _window, cx| {
                    this.key_generator.key_size = KEY_SIZES[i];
                    cx.notify();
                },
            )
        })
        .child(
            Button::new("keygen-generate-btn")
                .label("Generate key")
                .primary()
                .self_start()
                .on_click(cx.listener(|this, _, _window, cx| {
                    let bits = this.key_generator.key_size;
                    match crypto::generate_key(bits) {
                        Ok(key) => {
                            this.key_generator.generated_key = key.clone();
                            this.key_generator.generated_bits = bits;
                            this.add_key_history(key, (bits / 8) as usize);
                        }
                        Err(e) => {
                            this.key_generator.generated_key = format!("Error: {}", e.message);
                        }
                    }
                    cx.notify();
                })),
        )
        .children((!k.generated_key.is_empty()).then(|| {
            result_tile(
                cx,
                "Generated key",
                format!("{} ({} bit)", k.generated_key, k.generated_bits),
                "keygen-copy-btn",
            )
        }))
        .children(key_history_list(app, cx))
}

pub fn copy_button(id: &'static str, value: String) -> impl IntoElement {
    Button::new(id)
        .icon(IconName::Copy)
        .tooltip("Copy")
        .ghost()
        .xsmall()
        .on_click(move |_, _window, cx| {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.clone()));
        })
}

pub fn key_history_list(
    app: &CrittoUtil,
    cx: &mut Context<CrittoUtil>,
) -> Option<impl IntoElement> {
    (!app.key_history.is_empty()).then(|| {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("History"),
            )
            .children(app.key_history.iter().enumerate().map(|(i, entry)| {
                let value = entry.name.clone();
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .p_3()
                    .bg(cx.theme().secondary)
                    .corner_radii(Corners::all(px(10.0)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().text_sm().child(entry.name.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{} bit", entry.bits)),
                            ),
                    )
                    .child(copy_button_dyn(format!("keygen-hist-copy-{i}"), value))
            }))
    })
}

fn copy_button_dyn(id: String, value: String) -> impl IntoElement {
    Button::new(gpui::ElementId::from(id))
        .icon(IconName::Copy)
        .tooltip("Copy")
        .ghost()
        .xsmall()
        .on_click(move |_, _window, cx| {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.clone()));
        })
}
