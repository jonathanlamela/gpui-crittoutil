use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use gpui_component::ActiveTheme as _;
use gpui_component::Disableable as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;

use crate::app::CrittoUtil;

/// Agentic-mode side panel: a chat with a local LM Studio model that can call
/// this app's own crypto tools. Slides in next to the content area when the
/// top bar's "Agent" toggle is on.
pub fn render(
    app: &CrittoUtil,
    _window: &mut Window,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    let a = &app.agent;

    div()
        .id("agent-panel")
        .flex()
        .flex_col()
        .w(gpui::rems(22.0))
        .h_full()
        .flex_shrink_0()
        .border_l_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(div().text_sm().font_weight(gpui::FontWeight::BOLD).child("Agent"))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("Local model via {}", a.base_url)),
                ),
        )
        .child({
            let mut messages = div()
                .id("agent-messages")
                .flex()
                .flex_col()
                .flex_1()
                .gap_3()
                .p_3()
                .overflow_y_scroll();
            for message in visible_messages(a) {
                messages = messages.child(message_bubble(message, cx));
            }
            if a.is_running {
                messages = messages.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Thinking…"),
                );
            }
            messages
        })
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .border_t_1()
                .border_color(cx.theme().border)
                .child(Input::new(&a.input).cleanable(true))
                .child(
                    Button::new("agent-send-btn")
                        .label("Send")
                        .primary()
                        .disabled(a.is_running)
                        .self_start()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.send_agent_message(window, cx);
                        })),
                ),
        )
}

/// Only user/assistant turns are worth showing — tool-call plumbing (the
/// `tool_calls` request and the raw JSON `tool` results) is an implementation
/// detail of the agent loop, not something the user needs to read.
fn visible_messages(agent: &crate::app::AgentState) -> impl Iterator<Item = &crate::agent::ChatMessage> {
    agent
        .messages
        .iter()
        .filter(|m| (m.role == "user" || m.role == "assistant") && m.content.is_some())
}

fn message_bubble(message: &crate::agent::ChatMessage, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let is_user = message.role == "user";
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .when(is_user, |this| this.bg(cx.theme().secondary))
        .rounded(cx.theme().radius)
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(cx.theme().muted_foreground)
                .child(if is_user { "You" } else { "Agent" }),
        )
        .child(
            div()
                .text_sm()
                .child(message.content.clone().unwrap_or_default()),
        )
}
