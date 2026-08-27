use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _,
    Styled, div, prelude::FluentBuilder as _,
};
use gpui_component::Disableable as _;
use gpui_component::IconName;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Textarea;
use gpui_component::text::markdown;
use gpui_component::{Icon, Sizable as _};

use crate::app::CrittoUtil;
use agent::ChatMessage;

pub fn sheet_content_with_data(
    entity: &Entity<CrittoUtil>,
    messages: Vec<ChatMessage>,
    input: gpui::Entity<gpui_component::input::TextareaState>,
    is_running: bool,
    expanded: std::collections::HashSet<usize>,
    _cx: &mut App,
) -> impl IntoElement {
    let entity_send = entity.clone();
    let entity_toggle = entity.clone();
    let display_items = build_display_items(&messages);
    div()
        .flex()
        .flex_col()
        .size_full()
        .gap_2()
        .px_3()
        .pb_3()
        .pt_1()
        .child({
            let mut list = div()
                .id("agent-sheet-messages")
                .flex()
                .flex_col()
                .flex_1()
                .gap_2()
                .overflow_y_scroll();
            for item in display_items {
                match item {
                    DisplayItem::Message { role, content } => {
                        let is_user = role == "user";
                        list = list.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .p_3()
                                .rounded(gpui::px(8.0))
                                .when(is_user, |d| {
                                    d.bg(gpui::hsla(240.0 / 360.0, 0.05, 0.96, 1.0))
                                })
                                .when(!is_user, |d| {
                                    d.bg(gpui::hsla(0.0, 0.0, 1.0, 1.0))
                                        .border_1()
                                        .border_color(gpui::hsla(220.0 / 360.0, 0.13, 0.91, 1.0))
                                })
                                .shadow(vec![gpui::BoxShadow {
                                    color: gpui::hsla(0.0, 0.0, 0.0, 0.07),
                                    offset: gpui::point(gpui::px(0.0), gpui::px(2.0)),
                                    blur_radius: gpui::px(8.0),
                                    spread_radius: gpui::px(0.0),
                                    inset: false,
                                }])
                                .child(div().text_xs().font_weight(gpui::FontWeight::BOLD).child(
                                    if is_user {
                                        "You".to_string()
                                    } else {
                                        "Agent".to_string()
                                    },
                                ))
                                .child(div().text_sm().child(markdown(content))),
                        );
                    }
                    DisplayItem::Tools { group_index, calls } => {
                        let expanded_here = expanded.contains(&group_index);
                        let names = calls
                            .iter()
                            .map(|c| c.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let ec = entity_toggle.clone();
                        let mut container = div()
                            .flex()
                            .flex_col()
                            .rounded(gpui::px(8.0))
                            .border_1()
                            .border_color(gpui::hsla(220.0 / 360.0, 0.13, 0.91, 1.0))
                            .child(
                                div()
                                    .id(("agent-sheet-tool", group_index))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py_2()
                                    .text_xs()
                                    .child(Icon::new(IconName::Settings2).xsmall())
                                    .child(div().flex_1().truncate().child(names))
                                    .child(
                                        Icon::new(if expanded_here {
                                            IconName::ChevronDown
                                        } else {
                                            IconName::ChevronRight
                                        })
                                        .xsmall(),
                                    )
                                    .on_click(move |_, _, cx| {
                                        ec.update(cx, |this, cx| {
                                            this.toggle_tool_group(group_index, cx)
                                        });
                                    }),
                            );
                        if expanded_here {
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
                                                .p_2()
                                                .child(pretty_json(&call.arguments)),
                                        )
                                        .children(call.result.as_ref().map(|r| {
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap_1()
                                                .child(div().text_xs().child("Result"))
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_family("monospace")
                                                        .p_2()
                                                        .child(pretty_json(r)),
                                                )
                                        })),
                                );
                            }
                            container = container.child(body);
                        }
                        list = list.child(container);
                    }
                }
            }
            if is_running {
                list = list.child(div().text_xs().child("Thinking…"));
            }
            list
        })
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .border_t_1()
                .border_color(gpui::hsla(220.0 / 360.0, 0.13, 0.91, 1.0))
                .pt_3()
                .child(Textarea::new(&input).w_full().h(gpui::rems(4.5)))
                .child(
                    div().flex().child(
                        Button::new("agent-sheet-send")
                            .icon(IconName::ArrowRight)
                            .label("Send")
                            .primary()
                            .small()
                            .disabled(is_running)
                            .on_click({
                                let e = entity_send.clone();
                                move |_, window, cx| {
                                    e.update(cx, |this, cx| this.send_agent_message(window, cx));
                                }
                            }),
                    ),
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
