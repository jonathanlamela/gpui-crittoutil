pub mod sidebar;
pub mod home;
pub mod converter;
pub mod key_generator;
pub mod encrypter;
pub mod decrypter;
pub mod file_hasher;

use gpui::{Context, IntoElement, ParentElement, SharedString, Styled, Window, div};
use gpui_component::ActiveTheme as _;
use gpui_component::Disableable as _;
use gpui_component::Sizable as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;

use crate::app::CrittoUtil;
use crate::ui::key_picker::open_key_picker;

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
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child(label.to_string()))
                .child(
                    Button::new(SharedString::from(picker_id))
                        .label(picker_label)
                        .ghost()
                        .xsmall()
                        .disabled(!has_history)
                        .on_click(cx.listener(move |_this, _, window, cx| {
                            let entity = cx.entity();
                            open_key_picker(&entity, window, cx, picker_label, keys.clone(), on_pick.clone());
                        })),
                ),
        )
        .child(input)
}
