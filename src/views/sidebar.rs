use gpui::{
    BoxShadow, Context, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement as _, Styled, Window, div, point, prelude::FluentBuilder as _, px,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::{ActiveTheme as _, Icon, IconName, Sizable as _};

use crate::app::{CrittoUtil, Route};

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

/// Always-visible left navigation sidebar with the 6 top-level screens.
/// Hand-rolled rather than built on gpui-component's `Sidebar`/`SidebarMenu`
/// components — those bake in their own hover/overflow behavior (header
/// hover highlight, `overflow: hidden` clipping the panel's own box-shadow)
/// that fought the floating-card look this app wants, so plain divs give
/// full control instead.
pub fn render(
    app: &CrittoUtil,
    _window: &mut Window,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    let radius = px(10.0);

    // The shadow is drawn on this unclipped outer wrapper (shaped with the
    // same radius), since the inner panel needs `overflow: hidden` to clip
    // its own contents to its rounded corners — which would also clip a
    // shadow painted on that same element.
    div()
        .flex_shrink_0()
        .mt_8()
        .mx_4()
        .mb_4()
        .rounded(radius)
        .shadow(vec![
            BoxShadow {
                color: cx.theme().foreground.opacity(0.2),
                offset: point(px(0.0), px(4.0)),
                blur_radius: px(16.0),
                spread_radius: px(0.0),
                inset: false,
            },
            BoxShadow {
                color: cx.theme().foreground.opacity(0.12),
                offset: point(px(0.0), px(1.0)),
                blur_radius: px(3.0),
                spread_radius: px(0.0),
                inset: false,
            },
        ])
        .child(
            // 1px white "bezel" ring around the grey panel — the macOS-style
            // inset look: white background, a hairline margin, then the
            // panel's own color inside.
            div()
                .w(gpui::rems(13.0))
                .h_full()
                .overflow_hidden()
                .rounded(radius)
                .bg(cx.theme().background)
                .p(px(1.0))
                .child(
                    div()
                        .id("sidebar")
                        .flex()
                        .flex_col()
                        .size_full()
                        .overflow_hidden()
                        .rounded(radius)
                        .bg(cx.theme().sidebar)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .p_3()
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(cx.theme().sidebar_foreground)
                                        .child("CrittoUtil"),
                                )
                                .child(
                                    Button::new("toggle-agent-mode")
                                        .icon(
                                            Icon::new(IconName::Bot).text_color(if app.agent.open {
                                                cx.theme().primary_foreground
                                            } else {
                                                cx.theme().sidebar_foreground
                                            }),
                                        )
                                        .tooltip("Agent")
                                        .small()
                                        .map(|btn| if app.agent.open { btn.primary() } else { btn.outline() })
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            this.toggle_agent(cx);
                                        })),
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
                                .gap_1()
                                .px_2()
                                .pb_2()
                                .overflow_y_scroll()
                                .children(items)
                        })
                        .child(
                            div()
                                .id("sidebar-close-session")
                                .flex()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .py_2()
                                .m_2()
                                .rounded(px(8.0))
                                .text_sm()
                                .text_color(cx.theme().sidebar_foreground.opacity(0.7))
                                .hover(|this| this.bg(cx.theme().muted))
                                .child(Icon::new(IconName::ArrowLeft))
                                .child("All sessions")
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.close_session(cx);
                                })),
                        ),
                ),
        )
}

fn nav_item(app: &CrittoUtil, route: Route, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let active = app.route == route;
    let id: ElementId = format!("nav-{route:?}").into();
    let icon_color = if active {
        cx.theme().sidebar_accent_foreground
    } else {
        cx.theme().sidebar_foreground.opacity(0.7)
    };

    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_2()
        .rounded(px(8.0))
        .text_sm()
        .when(active, |this| {
            this.bg(cx.theme().sidebar_accent)
                .text_color(cx.theme().sidebar_accent_foreground)
        })
        .when(!active, |this| {
            this.text_color(cx.theme().sidebar_foreground)
                .hover(|this| this.bg(cx.theme().muted))
        })
        .child(Icon::new(route_icon(route)).text_color(icon_color))
        .child(route.label())
        .on_click(cx.listener(move |this, _, _window, cx| this.navigate(route, cx)))
}
