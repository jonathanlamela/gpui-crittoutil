use gpui::{Context, InteractiveElement, IntoElement, ParentElement, PathPromptOptions, Styled, Window, div};
use gpui_component::ActiveTheme as _;
use gpui_component::button::{Button, ButtonVariants as _};

use crate::app::CrittoUtil;
use crate::crypto;

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn render(app: &CrittoUtil, _window: &mut Window, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let f = &app.file_hasher;

    div()
        .id("file-hasher-view")
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
                .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).child("File hasher"))
                .child(
                    Button::new("filehasher-clear-btn")
                        .label("Clear")
                        .ghost()
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.file_hasher.filename.clear();
                            this.file_hasher.filesize = 0;
                            this.file_hasher.hash.clear();
                            cx.notify();
                        })),
                ),
        )
        .child(
            Button::new("filehasher-pick-btn")
                .label("Choose a file…")
                .primary()
                .w_full()
                .on_click(cx.listener(|this, _, window, cx| {
                    let _ = this;
                    let paths = cx.prompt_for_paths(PathPromptOptions {
                        files: true,
                        directories: false,
                        multiple: false,
                        prompt: None,
                    });
                    let view = cx.entity();
                    cx.spawn_in(window, async move |_, window| {
                        let path = paths.await.ok()?.ok()??.into_iter().next()?;
                        window
                            .update(|_window, cx| {
                                view.update(cx, |this, cx| {
                                    match std::fs::read(&path) {
                                        Ok(bytes) => {
                                            this.file_hasher.filename = path
                                                .file_name()
                                                .map(|n| n.to_string_lossy().to_string())
                                                .unwrap_or_default();
                                            this.file_hasher.filesize = bytes.len() as u64;
                                            this.file_hasher.hash = crypto::hash_md5_bytes(&bytes);
                                        }
                                        Err(e) => {
                                            this.file_hasher.filename = String::new();
                                            this.file_hasher.filesize = 0;
                                            this.file_hasher.hash = format!("Error reading file: {e}");
                                        }
                                    }
                                    cx.notify();
                                })
                            })
                            .ok()
                    })
                    .detach();
                })),
        )
        .child(if !f.filename.is_empty() {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().secondary)
                .child(div().text_sm().font_weight(gpui::FontWeight::BOLD).child(f.filename.clone()))
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child(format_size(f.filesize)))
                .into_any_element()
        } else {
            div().into_any_element()
        })
        .child(if !f.hash.is_empty() {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().secondary)
                .child(div().text_xs().font_weight(gpui::FontWeight::BOLD).child("MD5 hash"))
                .child(div().text_sm().child(f.hash.clone()))
                .child(super::key_generator::copy_button("filehasher-copy-btn", f.hash.clone()))
                .into_any_element()
        } else {
            div().into_any_element()
        })
}
