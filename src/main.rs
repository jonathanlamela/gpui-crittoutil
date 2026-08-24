mod app;
mod converter;
mod crypto;
mod crypto_meta;
mod home_search;
mod theme;
mod ui;
mod views;

use gpui::{Pixels, Size, *};
use gpui_component::Root;

use app::CrittoUtil;

fn main() {
    let gpui_app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    gpui_app.run(move |cx| {
        // Must be called before using any GPUI Component features.
        gpui_component::init(cx);

        // Load and apply the custom warm-neutral theme.
        theme::init(cx);

        cx.spawn(async move |cx| {
            let mut win_options = WindowOptions::default();

            win_options.window_min_size = Some(Size::new(Pixels::from(900.0), Pixels::from(640.0)));

            win_options.titlebar = Some(TitlebarOptions {
                title: Some(SharedString::from("CrittoUtil")),
                appears_transparent: true,
                traffic_light_position: None,
            });

            cx.open_window(win_options, |window, cx| {
                let view = cx.new(|cx| CrittoUtil::new(window, cx));

                window.on_window_should_close(cx, |_window, cx| {
                    cx.quit();
                    true
                });

                // The outermost view of a window must be wrapped in Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");

            cx.update(|cx| cx.activate(true));
        })
        .detach();
    });
}
