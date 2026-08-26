pub mod agent_panel;
pub mod converter;
pub mod decrypter;
pub mod encrypter;
pub mod file_hasher;
pub mod home;
pub mod key_generator;
pub mod session_picker;
pub mod sidebar;

use gpui::{Context, Corners, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::Disableable as _;
use gpui_component::IconName;
use gpui_component::Sizable as _;
use gpui_component::StyledExt;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;
use gpui_component::radio::{Radio, RadioGroup};

use crate::app::CrittoUtil;
use crate::ui::key_picker::open_key_picker;

/// A labeled result tile with a copy-to-clipboard icon button next to the
/// value — the shared "secondary output" treatment (rounded, `theme.secondary`
/// background, no border) used across Converter/Encrypter/Decrypter/Key
/// Generator/File Hasher for whatever the screen just produced.

pub fn result_tile(
    cx: &mut Context<CrittoUtil>,
    title: &str,
    value: String,
    copy_id: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .bg(cx.theme().secondary)
        .border_1()
        .border_color(cx.theme().border)
        .corner_radii(Corners::all(px(10.0)))
        .shadow(vec![gpui::BoxShadow {
            color: cx.theme().foreground.opacity(0.04),
            offset: gpui::point(px(0.0), px(1.0)),
            blur_radius: px(8.0),
            spread_radius: px(0.0),
            inset: false,
        }])
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(cx.theme().muted_foreground)
                .child(title.to_uppercase()),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().text_sm().child(value.clone()))
                .child(
                    Button::new(copy_id.into())
                        .icon(IconName::Copy)
                        .tooltip("Copy")
                        .ghost()
                        .xsmall()
                        .on_click(move |_, _window, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.clone()));
                        }),
                ),
        )
}

/// A labeled horizontal group of mutually-exclusive options, backed by the
/// library's standard `RadioGroup`/`Radio` components (used for the
/// algorithm/type/key-size pickers that used to be rows of plain buttons).
pub fn radio_row(
    app: &CrittoUtil,
    cx: &mut Context<CrittoUtil>,
    label: &str,
    group_id: &'static str,
    labels: Vec<String>,
    selected_index: Option<usize>,
    on_pick: impl Fn(&mut CrittoUtil, usize, &mut Window, &mut Context<CrittoUtil>) + 'static,
) -> impl IntoElement {
    let _ = app;
    let entity = cx.entity();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
        .child(
            RadioGroup::horizontal(group_id)
                .selected_index(selected_index)
                .children(
                    labels
                        .into_iter()
                        .enumerate()
                        .map(|(i, l)| Radio::new(format!("{group_id}-{i}")).label(l)),
                )
                .on_click(move |idx: &usize, window, cx| {
                    let idx = *idx;
                    entity.update(cx, |this, cx| on_pick(this, idx, window, cx));
                }),
        )
}

/// A labeled input field with a "pick from key history" button that opens a
/// dialog listing the shared key history (used for key/IV fields in the
/// Encrypter/Decrypter views).
pub fn field_with_picker(
    app: &CrittoUtil,
    cx: &mut Context<CrittoUtil>,
    label: &str,
    input: Input,
    picker_id: &'static str,
    picker_label: &'static str,
    on_pick: impl Fn(&mut CrittoUtil, &str, &mut Window, &mut Context<CrittoUtil>) + Clone + 'static,
) -> impl IntoElement {
    let has_history = !app.key_history.is_empty();
    let keys = app.key_history.clone();

    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(label.to_string()),
                )
                .child(
                    Button::new(SharedString::from(picker_id))
                        .label(picker_label)
                        .ghost()
                        .xsmall()
                        .disabled(!has_history)
                        .on_click(cx.listener(move |_this, _, window, cx| {
                            let entity = cx.entity();
                            open_key_picker(
                                &entity,
                                window,
                                cx,
                                picker_label,
                                keys.clone(),
                                on_pick.clone(),
                            );
                        })),
                ),
        )
        .child(input)
}
