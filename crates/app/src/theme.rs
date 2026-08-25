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
    cx.refresh_windows();
}
