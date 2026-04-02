use std::ops::Range;

use gpui::{
    AbsoluteLength, App, Bounds, Context, CursorStyle, Element, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId,
    InteractiveElement, IntoElement, LayoutId, MouseButton, PaintQuad, ParentElement, Pixels,
    Render, ShapedLine, SharedString, Style, Styled, Subscription, TextRun, UTF16Selection,
    UnderlineStyle, Window, div, fill, point, px,
};

use crate::{
    app::theme::ActiveTheme, loader::utils::ExecVariable,
    ui::search_bar::builder::TextInputBuilder, utils::paths::get_nth_path_completion,
};

pub mod actions;
mod builder;

// Implement event for mode change
pub struct EmptyBackspace;
impl EventEmitter<EmptyBackspace> for TextInput {}

pub struct TextInput {
    pub scope: Option<&'static str>,
    pub focus_handle: FocusHandle,
    pub content: SharedString,
    pub placeholder: SharedString,
    pub selected_range: Range<usize>,
    pub selection_reversed: bool,
    pub marked_range: Option<Range<usize>>,
    pub last_layout: Option<ShapedLine>,
    pub last_bounds: Option<Bounds<Pixels>>,
    pub is_selecting: bool,
    pub variable: Option<ExecVariable>,
    pub ghost_text: Option<String>,
    pub _sub: Option<Subscription>,
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();

        // update eventual completion
        self.refresh_ghost_text();

        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;

        assert_eq!(last_layout.text, self.content);
        let utf8_index = last_layout.index_for_x(point.x - line_point.x)?;
        Some(self.offset_to_utf16(utf8_index))
    }
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

struct TextElementRequestLayoutState {
    l: ShapedLine,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = TextElementRequestLayoutState;
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let input = self.input.read(cx);
        let content: SharedString = match &input.variable {
            Some(ExecVariable::PasswordInput(_)) => {
                "•".repeat(input.content.chars().count()).into()
            }
            _ => input.content.clone().into(),
        };
        let style = window.text_style();
        let theme = cx.global::<ActiveTheme>().0.clone();

        let (mut display_text, text_color) = if content.is_empty() {
            (input.placeholder.clone(), theme.text_placeholder)
        } else {
            (content.clone(), style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let completion_run = if let Some(completion) = &input.ghost_text {
            display_text = SharedString::from(format!("{display_text}{completion}"));
            Some(TextRun {
                len: completion.len(),
                font: style.font(),
                color: text_color.alpha(0.3),
                background_color: None,
                underline: None,
                strikethrough: None,
            })
        } else {
            None
        };

        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                Some(TextRun {
                    len: marked_range.start,
                    ..run.clone()
                }),
                Some(TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                }),
                Some(TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                }),
                completion_run,
            ]
            .into_iter()
            .filter_map(|run| run)
            .filter(|run| run.len > 0)
            .collect()
        } else {
            if let Some(completion_run) = completion_run {
                vec![run, completion_run]
            } else {
                vec![run]
            }
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        // Update style
        let mut style = Style::default();
        style.size.width = gpui::Length::Definite(gpui::DefiniteLength::Absolute(
            AbsoluteLength::Pixels(line.width + Pixels::from(2.0)),
        ));
        style.size.height = window.line_height().into();

        (
            window.request_layout(style, [], cx),
            Self::RequestLayoutState { l: line },
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let mut selected_range = input.selected_range.clone();
        let mut cursor = input.cursor_offset();

        let theme = cx.global::<ActiveTheme>().0.clone();

        // handle password fields
        if let Some(ExecVariable::PasswordInput(_)) = &input.variable {
            cursor = input.content[..cursor].chars().count() * "•".len();

            let start = input.content[..selected_range.start].chars().count() * "•".len();
            let end = input.content[..selected_range.end].chars().count() * "•".len();
            selected_range = start..end;
        }

        // Cached from request layout
        let line = &request_layout.l;

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        gpui::Size {
                            width: px(2.),
                            height: bounds.bottom() - bounds.top(),
                        },
                    ),
                    theme.cursor,
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    theme.selection,
                )),
                None,
            )
        };
        PrepaintState {
            line: Some(line.clone()),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let line = prepaint.line.take().unwrap();
        line.paint(
            bounds.origin,
            window.line_height(),
            gpui::TextAlign::Left,
            None,
            window,
            cx,
        )
        .unwrap();

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        div()
            .flex()
            .key_context(self.scope.unwrap_or("TextInput"))
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::complete))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_all))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .text_color(theme.secondary_text)
            .w_auto()
            .child(if self.variable.is_some() {
                div()
                    .line_height(px(12.))
                    .text_size(px(12.))
                    .h(px(20. + 4. * 2.)) // 38px
                    .p(px(4.))
                    .px(px(7.))
                    .w_auto()
                    .flex()
                    .flex_none()
                    .items_center()
                    .border(px(1.))
                    .border_color(theme.border)
                    .rounded_md()
                    .min_w(px(20.))
                    .font_family(theme.font_family.clone())
                    .child(TextElement { input: cx.entity() })
            } else {
                div()
                    .line_height(px(16.))
                    .text_size(px(16.))
                    .h(px(30. + 4. * 2.)) // 38px
                    .p(px(4.))
                    .w_auto()
                    .flex()
                    .flex_none()
                    .items_center()
                    .min_w(px(20.))
                    .font_family(theme.font_family.clone())
                    .child(TextElement { input: cx.entity() })
            })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TextInput {
    pub fn builder() -> TextInputBuilder {
        TextInputBuilder::new()
    }
    pub(super) fn refresh_ghost_text(&mut self) {
        self.ghost_text = match &self.variable {
            Some(ExecVariable::PathInput(inner)) => {
                get_nth_path_completion(self.content.as_str(), inner.index)
            }
            _ => None,
        };
    }
}
