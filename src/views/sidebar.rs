use gpui::{Context, IntoElement, ParentElement, Styled, Window, div};
use gpui_component::ActiveTheme as _;
use gpui_component::sidebar::{Sidebar, SidebarHeader, SidebarMenu, SidebarMenuItem};
use gpui_component::{Icon, IconName};

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

/// Always-visible left navigation sidebar with the 6 top-level screens, built
/// from gpui-component's standard `Sidebar`/`SidebarMenu` components (collapse
/// animation disabled — not needed on desktop, see task spec).
pub fn render(
    app: &CrittoUtil,
    _window: &mut Window,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    let entity = cx.entity();

    let menu = SidebarMenu::new().children(Route::ALL.into_iter().map(|route| {
        let entity = entity.clone();
        SidebarMenuItem::new(route.label())
            .icon(Icon::new(route_icon(route)))
            .active(app.route == route)
            .on_click(move |_, _window, cx| {
                entity.update(cx, |this, cx| this.navigate(route, cx));
            })
    }));

    Sidebar::new("sidebar")
        .collapsible(false)
        .w(gpui::rems(13.0))
        .h_full()
        .flex_shrink_0()
        .header(
            SidebarHeader::new().child(
                div()
                    .text_base()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(cx.theme().foreground)
                    .child("CrittoUtil"),
            ),
        )
        .child(menu)
}
