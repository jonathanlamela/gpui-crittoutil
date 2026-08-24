use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _,
    Styled, Window, div,
};
use gpui_component::ActiveTheme as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;

use crate::app::CrittoUtil;
use crate::converter::{self, ConvType};
use crate::views::{radio_row, result_tile};

pub fn render(
    app: &CrittoUtil,
    _window: &mut Window,
    cx: &mut Context<CrittoUtil>,
) -> impl IntoElement {
    let c = &app.converter;

    div()
        .id("converter-view")
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .size_full()
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Converter"),
                )
                .child(
                    Button::new("converter-clear-btn")
                        .label("Clear")
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.converter
                                .input
                                .update(cx, |s, cx| s.set_value("", window, cx));
                            this.converter.output.clear();
                            this.converter.input_error.clear();
                            cx.notify();
                        })),
                ),
        )
        .child(Input::new(&c.input).cleanable(true))
        .children(err_text(&c.input_error, cx))
        .child(type_row(
            app,
            "Convert from",
            "converter-from",
            ConvType::ALL.to_vec(),
            c.from_type,
            cx,
            |this, t, _window, cx| {
                this.converter.from_type = t;
                if this.converter.to_type == t {
                    this.converter.to_type = t.first_other();
                }
                this.converter.output.clear();
                this.converter.input_error.clear();
                cx.notify();
            },
        ))
        .child(type_row(
            app,
            "Convert to",
            "converter-to",
            ConvType::ALL
                .into_iter()
                .filter(|t| *t != c.from_type)
                .collect(),
            c.to_type,
            cx,
            |this, t, _window, cx| {
                this.converter.to_type = t;
                this.converter.output.clear();
                cx.notify();
            },
        ))
        .child(
            Button::new("converter-convert-btn")
                .label("Convert")
                .primary()
                .self_start()
                .on_click(cx.listener(|this, _, _window, cx| {
                    let input = this.converter.input.read(cx).value().to_string();
                    let from = this.converter.from_type;
                    let to = this.converter.to_type;
                    match converter::validate_input(&input, from) {
                        Err(e) => {
                            this.converter.input_error = e;
                            this.converter.output.clear();
                        }
                        Ok(()) => match converter::convert(&input, from, to) {
                            Ok(out) => {
                                this.converter.input_error.clear();
                                this.converter.output = out;
                            }
                            Err(e) => {
                                this.converter.input_error = e;
                                this.converter.output.clear();
                            }
                        },
                    }
                    cx.notify();
                })),
        )
        .children((!c.output.is_empty()).then(|| {
            result_tile(cx, "Output", c.output.clone(), "converter-copy-btn")
        }))
}

fn err_text(msg: &str, cx: &mut Context<CrittoUtil>) -> Option<impl IntoElement> {
    (!msg.is_empty()).then(|| {
        div()
            .text_xs()
            .text_color(cx.theme().danger)
            .child(msg.to_string())
    })
}

fn type_row(
    app: &CrittoUtil,
    label: &'static str,
    group_id: &'static str,
    options: Vec<ConvType>,
    current: ConvType,
    cx: &mut Context<CrittoUtil>,
    on_pick: impl Fn(&mut CrittoUtil, ConvType, &mut Window, &mut Context<CrittoUtil>) + 'static,
) -> impl IntoElement {
    let selected_index = options.iter().position(|t| *t == current);
    let labels = options.iter().map(|t| t.label().to_string()).collect();
    radio_row(
        app,
        cx,
        label,
        group_id,
        labels,
        selected_index,
        move |this, idx, window, cx| {
            on_pick(this, options[idx], window, cx);
        },
    )
}
