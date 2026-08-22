use gpui::{Context, InteractiveElement, IntoElement, ParentElement, Styled, Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::{Icon, IconName};
use gpui_component::button::{Button, ButtonVariants as _};

use crate::app::{CrittoUtil, Route};
use crate::ui::style;

fn route_icon(route: Route) -> IconName {
    match route {
        Route::Home => IconName::LayoutDashboard,
        Route::Converter => IconName::Replace,
        Route::KeyGenerator => IconName::Cpu,
        Route::Encrypter => IconName::EyeOff,
        Route::Decrypter => IconName::Eye,
        Route::FileHasher => IconName::File,
    }
}

/// Always-visible left navigation sidebar with the 6 top-level screens.
/// Port of `App.vue`'s navigation rail, minus the expand/collapse animation
/// (not needed on desktop — see task spec).
pub fn render(
    app: &CrittoUtil,
    _window: &mut Window,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    style::surface(
        div()
            .id("sidebar")
            .flex()
            .flex_col()
            .w(gpui::rems(13.0))
            .h_full()
            .flex_shrink_0()
            .gap_1()
            .p_2(),
        cx,
        px(20.0),
    )
    .child(
        div()
            .flex()
            .gap_2()
            .px_2()
            .py_3()
            .mb_1()
            .child(div().size(px(10.0)).rounded_full().bg(cx.theme().primary))
            .child(
                div()
                    .text_base()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(cx.theme().foreground)
                    .child("CrittoUtil"),
            ),
    )
    .children({
        let mut items = Vec::new();
        for route in Route::ALL {
            items.push(nav_item(app, route, cx).into_any_element());
        }
        items
    })
}

fn nav_item(app: &CrittoUtil, route: Route, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let selected = app.route == route;
    let id: gpui::ElementId = format!("nav-{:?}", route).into();
    let icon_color = if selected {
        cx.theme().primary_foreground
    } else {
        cx.theme().muted_foreground
    };

    let btn = Button::new(id).w_full().justify_start().child(
        div()
            .w_full()
            .flex()
            .items_center()
            .justify_start()
            .gap_2()
            .child(Icon::new(route_icon(route)).text_color(icon_color))
            .child(route.label()),
    );
    let btn = if selected { btn.primary() } else { btn.ghost() };
    btn.on_click(cx.listener(move |this, _, _window, cx| {
        this.navigate(route, cx);
    }))
}
