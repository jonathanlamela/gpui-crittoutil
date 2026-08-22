use gpui::{Styled, px};
use gpui_component::ActiveTheme as _;

/// Opaque card surface: rounded corners, a subtle border and a soft shadow.
/// No transparency tricks — solid theme colors only, for a crisp, high-contrast look.
pub fn surface<E: Styled>(el: E, cx: &gpui::App, radius: gpui::Pixels) -> E {
    let theme = cx.theme();
    el.bg(theme.popover)
        .border_1()
        .border_color(theme.border)
        .rounded(radius)
        .shadow(vec![gpui::BoxShadow {
            color: theme.foreground.opacity(0.08),
            offset: gpui::point(px(0.0), px(6.0)),
            blur_radius: px(18.0),
            spread_radius: px(0.0),
            inset: false,
        }])
}

/// A card-sized surface with generous rounding, for the main content panel.
pub fn card<E: Styled>(el: E, cx: &gpui::App) -> E {
    surface(el, cx, px(20.0))
}
