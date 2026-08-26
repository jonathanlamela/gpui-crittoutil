use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use gpui_component::ActiveTheme as _;
use gpui_component::Disableable as _;
use gpui_component::IconName;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Textarea;
use gpui_component::text::markdown;
use gpui_component::{Icon, Sizable as _};

use agent::ChatMessage;
use crate::app::{AgentState, CrittoUtil};

/// Agentic-mode side panel: a chat with a local LM Studio model that can call
/// this app's own crypto tools. Slides in next to the content area when the
/// sidebar's bot-icon toggle is on. The conversation (`app.agent.messages`)
/// lives on the always-alive `CrittoUtil` entity, so it survives closing and
/// reopening this panel — it only resets when the app itself restarts.
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
        .absolute()
        .right_0()
        .top_0()
        .bottom_0()
        .w(gpui::rems(26.0))
        .pt(gpui::px(28.0))
        .bg(cx.theme().background)
        .border_l_1()
        .border_color(cx.theme().border)
        .shadow(vec![gpui::BoxShadow {
            color: gpui::hsla(0.0, 0.0, 0.0, 0.12),
            offset: gpui::point(gpui::px(-4.0), gpui::px(0.0)),
            blur_radius: gpui::px(20.0),
            spread_radius: gpui::px(0.0),
            inset: false,
        }])
        .child(
            div()
                .flex()
                .items_center()
                .p_3()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Agent"),
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
            for item in build_display_items(&a.messages) {
                messages = messages.child(match item {
                    DisplayItem::Message { role, content } => {
                        message_bubble(&role, &content, cx).into_any_element()
                    }
                    DisplayItem::Tools { group_index, calls } => {
                        tool_call_group(group_index, calls, a, cx).into_any_element()
                    }
                });
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
                .child(Textarea::new(&a.input).w_full().h(gpui::rems(4.5)))
                .child(
                    Button::new("agent-send-btn")
                        .icon(IconName::ArrowRight)
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

enum DisplayItem {
    Message {
        role: String,
        content: String,
    },
    Tools {
        group_index: usize,
        calls: Vec<ToolCallDisplay>,
    },
}

struct ToolCallDisplay {
    name: String,
    arguments: String,
    result: Option<String>,
}

/// Turn the raw message history into display items: plain user/assistant
/// bubbles, and any run of `role: "tool"` results collapsed into a single
/// collapsible block — whether they came from real `tool_calls` or the
/// text-narrated fallback in `agent::run_turn` (which has no structured call
/// of its own, only the `display` name/arguments stashed on the tool
/// message). Either way the model's own assistant turn is *also* shown as a
/// normal bubble beforehand if it had visible content (e.g. the narration
/// text in the fallback case) — the group is purely additive, showing what
/// was actually executed.
fn build_display_items(messages: &[ChatMessage]) -> Vec<DisplayItem> {
    let mut items = Vec::new();
    let mut i = 0;
    while i < messages.len() {
        let m = &messages[i];

        if m.role == "tool" {
            let start = i;
            let mut calls = Vec::new();
            while i < messages.len() && messages[i].role == "tool" {
                let t = &messages[i];
                let (name, arguments) = t
                    .display
                    .clone()
                    .unwrap_or_else(|| ("tool".to_string(), "{}".to_string()));
                calls.push(ToolCallDisplay {
                    name,
                    arguments,
                    result: t.content.clone(),
                });
                i += 1;
            }
            items.push(DisplayItem::Tools {
                group_index: start,
                calls,
            });
            continue;
        }

        if (m.role == "user" || m.role == "assistant")
            && m.content.as_deref().is_some_and(|c| !c.is_empty())
        {
            items.push(DisplayItem::Message {
                role: m.role.clone(),
                content: m.content.clone().unwrap_or_default(),
            });
        }
        i += 1;
    }
    items
}

fn pretty_json(raw: &str) -> String {
    serde_json::from_str::<serde_json::Value>(raw)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .unwrap_or_else(|_| raw.to_string())
}

fn message_bubble(role: &str, content: &str, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let is_user = role == "user";
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .when(is_user, |this| this.bg(cx.theme().secondary))
        .when(!is_user, |this| this.bg(cx.theme().background).border_1().border_color(cx.theme().border))
        .rounded(cx.theme().radius)
        .shadow(vec![gpui::BoxShadow {
            color: gpui::hsla(0.0, 0.0, 0.0, 0.07),
            offset: gpui::point(gpui::px(0.0), gpui::px(2.0)),
            blur_radius: gpui::px(8.0),
            spread_radius: gpui::px(0.0),
            inset: false,
        }])
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(cx.theme().muted_foreground)
                .child(if is_user { "You" } else { "Agent" }),
        )
        .child(div().text_sm().child(markdown(content.to_string())))
}

/// Collapsed by default: an icon + comma-joined tool names + chevron; click
/// to reveal each call's name, arguments, and result as formatted JSON —
/// same shape as the Tauri sibling app's `ToolCallGroup`.
fn tool_call_group(
    group_index: usize,
    calls: Vec<ToolCallDisplay>,
    agent: &AgentState,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    let expanded = agent.expanded_tool_calls.contains(&group_index);
    let names = calls
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let mut container = div()
        .flex()
        .flex_col()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .id(("agent-tool-group", group_index))
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .bg(cx.theme().muted)
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .hover(|this| this.text_color(cx.theme().foreground))
                .child(Icon::new(IconName::Settings2).xsmall())
                .child(div().flex_1().truncate().child(names))
                .child(
                    Icon::new(if expanded {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .xsmall(),
                )
                .on_click(cx.listener(move |this, _, _window, cx| {
                    this.toggle_tool_group(group_index, cx);
                })),
        );

    if expanded {
        let mut body = div().flex().flex_col().gap_2().p_3();
        for call in &calls {
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(call.name.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family("monospace")
                            .bg(cx.theme().muted)
                            .p_2()
                            .rounded(cx.theme().radius)
                            .child(pretty_json(&call.arguments)),
                    )
                    .children(call.result.as_ref().map(|result| {
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Result"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .bg(cx.theme().muted)
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .child(pretty_json(result)),
                            )
                    })),
            );
        }
        container = container.child(body);
    }

    container
}
