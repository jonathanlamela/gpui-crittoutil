use gpui::{Context, ElementId, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, px};
use gpui_component::{ActiveTheme as _, Icon, IconName};

use crate::app::CrittoUtil;
use home::Route;

pub fn route_icon(route: Route) -> IconName {
    match route {
        Route::Home => IconName::LayoutDashboard,
        Route::Converter => IconName::Replace,
        Route::KeyGenerator => IconName::Cpu,
        Route::Encrypter => IconName::EyeOff,
        Route::Decrypter => IconName::Eye,
        Route::FileHasher => IconName::File,
    }
}

pub fn render(
    app: &CrittoUtil,
    _window: &mut Window,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    div()
        .id("sidebar")
        .flex()
        .flex_col()
        .flex_shrink_0()
        .w(gpui::rems(13.5))
        .h_full()
        .bg(cx.theme().sidebar)
        .border_r_1()
        .border_color(cx.theme().sidebar_border)
        .pt(px(28.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .px_4()
                .pt_5()
                .pb_4()
                .border_b_1()
                .border_color(cx.theme().sidebar_border.opacity(0.6))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .size(px(28.0))
                                .rounded(px(7.0))
                                .bg(cx.theme().primary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new(IconName::Settings)
                                        .text_color(cx.theme().primary_foreground),
                                ),
                        )
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(cx.theme().sidebar_foreground)
                                .child("CrittoUtil"),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().sidebar_foreground.opacity(0.5))
                        .child("CRYPTO UTILITY  •  v1.0"),
                ),
        )
        .child({
            let mut items = Vec::new();
            for route in Route::ALL {
                items.push(nav_item(app, route, cx).into_any_element());
            }
            div()
                .id("sidebar-nav")
                .flex()
                .flex_col()
                .flex_1()
                .gap_0p5()
                .px_2()
                .pt_3()
                .pb_2()
                .overflow_y_scroll()
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(cx.theme().sidebar_foreground.opacity(0.38))
                        .px_2()
                        .pb_2()
                        .child("NAVIGATION"),
                )
                .children(items)
        })
        .child(
            div()
                .mx_2()
                .mt_2()
                .mb_3()
                .h(px(1.0))
                .bg(cx.theme().sidebar_border.opacity(0.6)),
        )
        .child(
            div()
                .id("sidebar-close-session")
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .mx_2()
                .mb_3()
                .rounded(px(8.0))
                .text_xs()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(cx.theme().sidebar_foreground.opacity(0.6))
                .hover(|this| {
                    this.bg(cx.theme().sidebar_foreground.opacity(0.08))
                        .text_color(cx.theme().sidebar_foreground)
                })
                .child(Icon::new(IconName::ArrowLeft))
                .child("All sessions")
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.close_session(cx);
                })),
        )
}

fn nav_item(app: &CrittoUtil, route: Route, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let active = app.route == route;
    let id: ElementId = format!("nav-{route:?}").into();
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2p5()
        .px_3()
        .py_2()
        .rounded(px(8.0))
        .text_sm()
        .when(active, |this| {
            this.bg(cx.theme().sidebar_accent)
                .text_color(cx.theme().sidebar_accent_foreground)
        })
        .when(!active, |this| {
            this.text_color(cx.theme().sidebar_foreground.opacity(0.75))
                .hover(|this| {
                    this.bg(cx.theme().sidebar_foreground.opacity(0.07))
                        .text_color(cx.theme().sidebar_foreground)
                })
        })
        .child(Icon::new(route_icon(route)).text_color(if active {
            cx.theme().sidebar_accent_foreground
        } else {
            cx.theme().sidebar_foreground.opacity(0.55)
        }))
        .child(
            div()
                .font_weight(if active {
                    gpui::FontWeight::SEMIBOLD
                } else {
                    gpui::FontWeight::MEDIUM
                })
                .child(route.label()),
        )
        .on_click(cx.listener(move |this, _, _window, cx| this.navigate(route, cx)))
}
