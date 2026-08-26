use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _,
    Styled, Window, div, px,
};
use gpui_component::ActiveTheme as _;
use gpui_component::Icon;
use gpui_component::Sizable as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;

use crate::app::CrittoUtil;
use crate::views::sidebar::route_icon;
use home::Route;

const FEATURES: &[(Route, &str)] = &[
    (
        Route::Converter,
        "Convert text between plain text, binary, and Base64.",
    ),
    (
        Route::KeyGenerator,
        "Generate a random cryptographic key of a chosen bit size.",
    ),
    (
        Route::Encrypter,
        "Encrypt text with MD5, AES (CBC/ECB), or DES (CBC/ECB).",
    ),
    (
        Route::Decrypter,
        "Decrypt a Base64 payload with AES or DES.",
    ),
    (Route::FileHasher, "Pick a file and compute its MD5 hash."),
];

pub fn render(
    app: &CrittoUtil,
    _window: &mut Window,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    let query = app.home_search.read(cx).value().to_string();
    let suggestion = home::search(&query);

    div()
        .id("home-view")
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .size_full()
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Welcome back"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Search or pick a tool to get started."),
                ),
        )
        .child(Input::new(&app.home_search).large().cleanable(true))
        .children(match suggestion {
            Some(route) => Some(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child("Did you mean:")
                    .child(
                        Button::new("home-search-suggestion")
                            .label(route.label())
                            .primary()
                            .on_click(
                                cx.listener(move |this, _, _window, cx| this.navigate(route, cx)),
                            ),
                    )
                    .into_any_element(),
            ),
            None if !query.trim().is_empty() => Some(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("No matching feature found.")
                    .into_any_element(),
            ),
            None => None,
        })
        .child({
            let mut cards = Vec::new();
            for (route, desc) in FEATURES {
                cards.push(feature_card(*route, desc, cx).into_any_element());
            }
            div().flex().flex_col().gap_3().children(cards)
        })
}

fn feature_card(
    route: Route,
    description: &'static str,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    div()
        .id(gpui::ElementId::from(format!("home-card-{:?}", route)))
        .flex()
        .items_center()
        .gap_3()
        .p_4()
        .rounded(px(10.0))
        .bg(cx.theme().secondary)
        .border_1()
        .border_color(cx.theme().border)
        .hover(|this| {
            this.bg(cx.theme().secondary_hover)
                .border_color(cx.theme().primary.opacity(0.25))
        })
        .child(
            div()
                .size(px(36.0))
                .rounded(px(8.0))
                .bg(cx.theme().accent)
                .flex()
                .items_center()
                .justify_center()
                .child(Icon::new(route_icon(route)).text_color(cx.theme().accent_foreground)),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(route.label()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(description),
                ),
        )
        .on_click(cx.listener(move |this, _, _window, cx| this.navigate(route, cx)))
}
