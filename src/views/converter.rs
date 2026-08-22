use gpui::{Context, InteractiveElement, IntoElement, ParentElement, Styled, Window, div};
use gpui_component::ActiveTheme as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;

use crate::app::CrittoUtil;
use crate::converter::{self, ConvType};

pub fn render(app: &CrittoUtil, _window: &mut Window, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    let c = &app.converter;

    div()
        .id("converter-view")
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .size_full()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).child("Converter"))
                .child(
                    Button::new("converter-clear-btn")
                        .label("Clear")
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.converter.input.update(cx, |s, cx| s.set_value("", window, cx));
                            this.converter.output.clear();
                            this.converter.input_error.clear();
                            cx.notify();
                        })),
                ),
        )
        .child(Input::new(&c.input).cleanable(true))
        .child(err_text(&c.input_error, cx))
        .child(type_row("Convert from", ConvType::ALL.to_vec(), c.from_type, cx, |this, t, cx| {
            this.converter.from_type = t;
            if this.converter.to_type == t {
                this.converter.to_type = t.first_other();
            }
            this.converter.output.clear();
            this.converter.input_error.clear();
            cx.notify();
        }))
        .child(type_row(
            "Convert to",
            ConvType::ALL.into_iter().filter(|t| *t != c.from_type).collect(),
            c.to_type,
            cx,
            |this, t, cx| {
                this.converter.to_type = t;
                this.converter.output.clear();
                cx.notify();
            },
        ))
        .child(
            Button::new("converter-convert-btn")
                .label("Convert")
                .primary()
                .w_full()
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
        .child(if !c.output.is_empty() {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().secondary)
                .child(div().text_xs().font_weight(gpui::FontWeight::BOLD).child("Output"))
                .child(div().text_sm().child(c.output.clone()))
                .child(
                    Button::new("converter-copy-btn")
                        .label("Copy")
                        .ghost()
                        .on_click({
                            let value = c.output.clone();
                            move |_, _window, cx| {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.clone()));
                            }
                        }),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        })
}

fn err_text(msg: &str, cx: &mut Context<CrittoUtil>) -> impl IntoElement {
    if msg.is_empty() {
        div().into_any_element()
    } else {
        div().text_xs().text_color(cx.theme().danger).child(msg.to_string()).into_any_element()
    }
}

fn type_row(
    label: &'static str,
    options: Vec<ConvType>,
    current: ConvType,
    cx: &mut Context<CrittoUtil>,
    on_pick: impl Fn(&mut CrittoUtil, ConvType, &mut Context<CrittoUtil>) + 'static + Clone,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(label))
        .child(
            div().flex().gap_2().children(options.into_iter().map(|t| {
                let selected = t == current;
                let id: gpui::ElementId = format!("{}-{:?}", label, t).into();
                let on_pick = on_pick.clone();
                let btn = Button::new(id).label(t.label());
                let btn = if selected { btn.primary() } else { btn.outline() };
                btn.on_click(cx.listener(move |this, _, _window, cx| {
                    on_pick(this, t, cx);
                }))
            })),
        )
}
