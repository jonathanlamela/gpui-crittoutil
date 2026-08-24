use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _,
    Styled, Window, div,
};
use gpui_component::ActiveTheme as _;
use gpui_component::Icon;
use gpui_component::IconName;
use gpui_component::button::{Button, ButtonVariants as _};

use crate::app::CrittoUtil;
use crate::session;

/// The screen shown when no session is active: a "New session" action and
/// the list of previously-saved sessions to resume. This is the very first
/// thing the app shows on launch.
pub fn render(
    app: &CrittoUtil,
    _window: &mut Window,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    div()
        .id("session-picker")
        .flex()
        .flex_col()
        .items_center()
        .gap_6()
        .p_10()
        .pt(gpui::rems(6.0))
        .size_full()
        .bg(cx.theme().background)
        .text_color(cx.theme().foreground)
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .child(div().text_xl().font_weight(gpui::FontWeight::BOLD).child("CrittoUtil"))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Start a new session or resume a recent one."),
                ),
        )
        .child(
            Button::new("session-picker-new")
                .label("New session")
                .primary()
                .icon(IconName::Plus)
                .on_click(cx.listener(|this, _, _window, cx| this.create_session(cx))),
        )
        .child({
            let mut list = div().flex().flex_col().gap_2().w(gpui::rems(28.0));
            if app.sessions.is_empty() {
                list = list.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("No recent sessions yet."),
                );
            } else {
                list = list.child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(cx.theme().muted_foreground)
                        .child("Recent sessions"),
                );
                let mut rows = Vec::new();
                for session in &app.sessions {
                    rows.push(session_row(session, cx).into_any_element());
                }
                list = list.children(rows);
            }
            list
        })
}

fn session_row(session: &session::Session, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let id = session.id.clone();
    let key_count = session.key_history.len();

    div()
        .id(gpui::ElementId::from(format!("session-row-{}", session.id)))
        .flex()
        .items_center()
        .gap_3()
        .p_3()
        .bg(cx.theme().secondary)
        .rounded(cx.theme().radius)
        .hover(|this| this.bg(cx.theme().muted))
        .child(Icon::new(IconName::LayoutDashboard).text_color(cx.theme().muted_foreground))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .child(div().text_sm().font_weight(gpui::FontWeight::BOLD).child(session.name.clone()))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!(
                            "{} · {} key{}",
                            session::format_created_at(session.created_at_unix),
                            key_count,
                            if key_count == 1 { "" } else { "s" }
                        )),
                ),
        )
        .on_click(cx.listener(move |this, _, _window, cx| this.open_session(&id, cx)))
}
