use gpui::{App, Entity, ParentElement, SharedString, Styled, Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::WindowExt as _;
use gpui_component::button::{Button, ButtonVariants as _};

use crate::app::{CrittoUtil, KeyEntry};

/// Opens a modal dialog listing the shared key history so the user can pick one
/// entry to fill a key/IV field with, instead of a row of small inline buttons.
pub fn open_key_picker(
    entity: &Entity<CrittoUtil>,
    window: &mut Window,
    cx: &mut App,
    title: &'static str,
    keys: Vec<KeyEntry>,
    on_pick: impl Fn(&mut CrittoUtil, &str, &mut Window, &mut gpui::Context<CrittoUtil>) + Clone + 'static,
) {
    let entity = entity.clone();
    window.open_dialog(cx, move |dialog, _window, _cx| {
        let entity = entity.clone();
        let keys = keys.clone();
        let on_pick = on_pick.clone();
        dialog.title(title).w(px(480.0)).content(move |content, _window, cx| {
            if keys.is_empty() {
                return content.child(
                    div()
                        .p_3()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("No keys in history yet."),
                );
            }
            content
                .gap_1()
                .children(keys.iter().map(|k| {
                    let name = k.name.clone();
                    let label = format!("{name} ({} bit)", k.bits);
                    let entity = entity.clone();
                    let on_pick = on_pick.clone();
                    let id: SharedString = format!("key-picker-{name}").into();
                    Button::new(id)
                        .label(label)
                        .ghost()
                        .w_full()
                        .justify_start()
                        .on_click(move |_, window, cx| {
                            let name = name.clone();
                            entity.update(cx, |this, cx| on_pick(this, &name, window, cx));
                            window.close_dialog(cx);
                        })
                }))
        })
    });
}
