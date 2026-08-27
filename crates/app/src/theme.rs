use gpui::App;
use gpui_component::{Theme, ThemeMode, ThemeRegistry};

const CUSTOM_THEME_FILE: &str = include_str!("../../../themes/custom.json");

pub fn init(cx: &mut App) {
    let registry = ThemeRegistry::global_mut(cx);
    registry
        .load_themes_from_str(CUSTOM_THEME_FILE)
        .expect("failed to load custom theme");

    let config_light = registry
        .themes()
        .get("CrittoUtil Light")
        .cloned()
        .expect("custom theme not registered");

    let config_dark = registry
        .themes()
        .get("CrittoUtil Dark")
        .cloned()
        .expect("custom theme not registered");

    Theme::global_mut(cx).light_theme = config_light;
    Theme::global_mut(cx).dark_theme = config_dark;

    Theme::change(ThemeMode::Light, None, cx);

    // The agent sheet should behave like a full-height sidebar flush with the
    // window edges, not leave a title-bar-sized gap above it (gpui-component's
    // default reserves TITLE_BAR_HEIGHT so sheets don't overlap a title bar).
    Theme::global_mut(cx).sheet.margin_top = gpui::px(0.0);

    cx.refresh_windows();
}
