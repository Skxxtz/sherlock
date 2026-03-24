use std::sync::Arc;

use gpui::{
    AnyElement, Context, Element, Focusable, FontWeight, Image, ImageSource, InteractiveElement,
    IntoElement, MouseDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Window, div, hsla, img, list, prelude::FluentBuilder, px, relative, rgb,
};

use crate::{
    CONTEXT_MENU_BIND,
    launcher::children::{RenderableChild, RenderableChildDelegate},
    ui::{
        UIFunction,
        launcher::{LauncherView, views::EntityStyle},
        workspace::LauncherErrorEvent,
    },
    utils::config::ConfigGuard,
};

impl Render for LauncherView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle(cx))
            .flex()
            .flex_col()
            .size_full()
            .on_action(cx.listener(Self::selection_up))
            .on_action(cx.listener(Self::selection_down))
            .on_action(cx.listener(Self::selection_left))
            .on_action(cx.listener(Self::selection_right))
            .on_action(cx.listener(Self::next_var))
            .on_action(cx.listener(Self::prev_var))
            .on_action(cx.listener(Self::execute))
            .on_action(cx.listener(Self::quit))
            .on_action(cx.listener(Self::open_context))
            .child(self.render_search_bar())
            .when(!self.config_initialized, |this| {
                this.child(self.render_config_banner())
            })
            .child(self.render_mode_label())
            .child({
                match self.navigation.current().style {
                    EntityStyle::Grid { .. } => self.render_result_grid(cx).into_any_element(),
                    EntityStyle::Row { .. } => self.render_results(cx).into_any_element(),
                }
            })
            .child(self.render_status_bar(cx))
    }
}

impl LauncherView {
    fn render_search_bar(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .w_full()
            .items_center()
            .px_4()
            .py(px(4.))
            .gap_3()
            .child(div().text_color(rgb(0x888888)).child(""))
            .child(div().w_auto().child(self.text_input.clone()))
            .children(self.variable_input.iter().cloned())
            .border_b_2()
            .border_color(hsla(0., 0., 0.1882, 1.0))
    }

    fn render_config_banner(&self) -> impl IntoElement {
        div()
            .w_full()
            .px_4()
            .py(px(6.))
            .bg(hsla(0.11, 0.8, 0.12, 1.0))
            .border_b_1()
            .border_color(hsla(0.11, 0.9, 0.35, 1.0))
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(hsla(0.11, 1.0, 0.65, 1.0))
                    .child("⚠  Running with default config — run "),
            )
            .child(
                div()
                    .px(px(4.))
                    .py(px(1.))
                    .rounded_sm()
                    .bg(rgb(0x1e1e1e))
                    .text_size(px(11.0))
                    .text_color(rgb(0x89d4f5))
                    .font_family("monospace")
                    .child("sherlock init"),
            )
    }

    fn render_mode_label(&self) -> impl IntoElement {
        div()
            .px(px(14.))
            .py(px(4.))
            .text_size(px(14.))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(0x2e2e2e))
            .child(self.mode.display_str())
    }

    fn render_results(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (indices, data) = self
            .navigation
            .with_model(cx, |mdl| (mdl.filtered_indices.clone(), mdl.data.clone()));
        let EntityStyle::Row {
            state,
            selected_index,
        } = &self.navigation.current().style
        else {
            return div().id("results-container");
        };
        let context_open = self.context_idx.is_some();

        div()
            .id("results-container")
            .flex_1()
            .min_h_0()
            .px(px(10.))
            .child({
                let selected_idx = *selected_index;
                list(state.clone(), move |idx, _win, cx| {
                    let data_idx = match indices.get(idx) {
                        Some(&i) => i,
                        None => return div().into_any_element(),
                    };
                    let data_guard = data.read(cx);
                    let child = match data_guard.get(data_idx) {
                        Some(c) => c,
                        None => return div().into_any_element(),
                    };

                    Self::render_list_item(&child, idx, data_idx, selected_idx, context_open)
                })
                .size_full()
            })
            .child(self.render_context_menu())
    }

    fn render_result_grid(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (indices, data) = self
            .navigation
            .with_model(cx, |mdl| (mdl.filtered_indices.clone(), mdl.data.clone()));

        let style = &self.navigation.current().style;
        let EntityStyle::Grid {
            scroll_handle,
            selected_index,
            columns,
            ..
        } = style
        else {
            return div().id("results-container");
        };

        let selected_idx = *selected_index;
        let col_count = *columns;
        let context_open = self.context_idx.is_some();

        div()
            .id("results-container")
            .flex_1()
            .min_h_0()
            .px(px(10.))
            .child(
                gpui::uniform_list(
                    "emoji-grid",
                    (indices.len() + col_count - 1) / col_count,
                    move |range, _win, cx| {
                        range
                            .map(|row_idx| {
                                div()
                                    .flex()
                                    .flex_row()
                                    .w_full()
                                    .gap(px(2.0))
                                    .children((0..col_count).map(|col_idx| {
                                        let item_idx = row_idx * col_count + col_idx;

                                        if let Some(&data_idx) = indices.get(item_idx) {
                                            let data_guard = data.read(cx);
                                            if let Some(child) = data_guard.get(data_idx) {
                                                return div()
                                                    .w_0()
                                                    .flex_1()
                                                    .child(Self::render_list_item(
                                                        child,
                                                        item_idx,
                                                        data_idx,
                                                        selected_idx,
                                                        context_open,
                                                    ))
                                                    .into_any_element();
                                            }
                                        }
                                        // Empty cell for alignment
                                        div().flex_1().into_any_element()
                                    }))
                                    .into_any_element()
                            })
                            .collect::<Vec<_>>()
                    },
                )
                .gap(px(2.0))
                .track_scroll(scroll_handle)
                .size_full(),
            )
    }

    fn render_context_menu(&self) -> impl IntoElement {
        if let Some(active) = self.context_idx {
            div().inset_0().absolute().child(
                div()
                    .p(px(7.))
                    .bg(rgb(0x0F0F0F))
                    .border_color(hsla(0., 0., 0.1882, 1.0))
                    .border(px(1.))
                    .rounded_md()
                    .absolute()
                    .bottom(px(10.))
                    .right(px(10.))
                    .flex()
                    .flex_col()
                    .min_w(px(200.))
                    .gap(px(5.))
                    .children(self.context_actions.iter().enumerate().map(|(i, child)| {
                        let is_selected = i == active;
                        div()
                            .group("")
                            .rounded_md()
                            .relative()
                            .flex_1()
                            .flex()
                            .gap(px(10.))
                            .p(px(10.))
                            .cursor_pointer()
                            .text_color(if is_selected {
                                hsla(0.0, 0.0, 0.8, 1.0)
                            } else {
                                hsla(0.6, 0.0217, 0.3608, 1.0)
                            })
                            .text_size(px(13.))
                            .line_height(relative(1.0))
                            .items_center()
                            .bg(if is_selected {
                                hsla(0., 0., 0.149, 1.0)
                            } else {
                                hsla(0., 0., 0., 0.)
                            })
                            .hover(|s| {
                                if is_selected {
                                    s
                                } else {
                                    s.bg(hsla(0., 0., 0.12, 1.0))
                                }
                            })
                            .child(if let Some(icon) = child.icon.as_ref() {
                                img(Arc::clone(icon)).size(px(16.)).into_any_element()
                            } else {
                                img(ImageSource::Image(Arc::new(Image::empty())))
                                    .size(px(16.))
                                    .into_any_element()
                            })
                            .child(child.name.as_ref().unwrap().clone())
                    })),
            )
        } else {
            div()
        }
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h(px(30.))
            .line_height(px(30.))
            .w_full()
            .flex()
            .bg(hsla(0., 0., 0.098, 1.0))
            .border_t_1()
            .border_color(hsla(0., 0., 0.1882, 1.0))
            .px_5()
            .text_size(px(13.))
            .items_center()
            .text_color(hsla(0.6, 0.0217, 0.3608, 1.0))
            .child(String::from("Sherlock"))
            .when(!self.error_count.is_empty(), |this| {
                this.child(self.render_error_indicator(cx))
            })
            .child(div().flex_1())
            .child(self.render_context_hint(cx))
    }

    fn render_error_indicator(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h_full()
            .flex()
            .items_center()
            .gap(px(2.))
            .p(px(3.))
            .cursor_pointer()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, _, cx| {
                    cx.emit(LauncherErrorEvent::ShowErrors);
                }),
            )
            .when(self.error_count.warnings > 0, |this| {
                this.child(
                    div()
                        .h_full()
                        .px(px(5.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .rounded_sm()
                        .border_1()
                        .border_color(hsla(0.11, 0.8, 0.55, 0.25))
                        .text_color(rgb(0xef9f27))
                        .bg(hsla(0.11, 0.8, 0.1, 0.10))
                        .child(self.error_count.warnings.to_string()),
                )
            })
            .when(self.error_count.errors > 0, |this| {
                this.child(
                    div()
                        .h_full()
                        .px(px(5.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .rounded_sm()
                        .border_1()
                        .border_color(hsla(0.0, 0.7, 0.59, 0.25))
                        .text_color(hsla(0.0, 0.7, 0.59, 0.25))
                        .bg(hsla(0.0, 0.7, 0.1, 0.12))
                        .child(self.error_count.errors.to_string()),
                )
            })
    }

    fn render_context_hint(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(selected_idx) = self.navigation.selected_index() else {
            return div();
        };
        let has_actions = self.navigation.with_model_mut(cx, |mdl, cx| {
            let data = mdl.data.read(cx);
            mdl.filtered_indices
                .get(selected_idx)
                .and_then(|i| data.get(*i))
                .and_then(RenderableChild::actions)
                .map(|a| !a.is_empty())
                .unwrap_or(false)
        });

        if has_actions {
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(div().mr_1().child(SharedString::from("Additional Actions")))
                .children(get_context_key_parts().into_iter().map(keybind_box))
        } else {
            div()
        }
    }

    pub fn render_list_item(
        ad: &RenderableChild,
        idx: usize,
        data_idx: usize,
        selected_index: usize,
        context_open: bool,
    ) -> AnyElement {
        let is_selected = selected_index == idx;
        div()
            .id(data_idx)
            .w_full()
            .on_click(move |_, _, _| {})
            .child(
                div()
                    .group("")
                    .rounded_md()
                    .relative()
                    .mb(px(5.0))
                    .w_full()
                    .cursor_pointer()
                    .bg(if is_selected {
                        hsla(0., 0., 0.149, 1.0)
                    } else {
                        hsla(0., 0., 0., 0.)
                    })
                    .hover(|s| {
                        if is_selected || context_open {
                            s
                        } else {
                            s.bg(hsla(0., 0., 0.12, 1.0))
                        }
                    })
                    .child(ad.render(is_selected)),
            )
            .into_any_element()
    }
}

fn get_context_key_parts() -> Vec<String> {
    CONTEXT_MENU_BIND
        .get_or_init(|| {
            ConfigGuard::read()
                .ok()
                .and_then(|config| {
                    config
                        .keybinds
                        .iter()
                        .find(|(_, func)| **func == UIFunction::ToggleContext)
                        .map(|(name, _)| name.clone())
                })
                .unwrap_or_else(|| "ctrl-l".to_string())
        })
        .split('-')
        .map(|part| match part {
            "ctrl" => "⌃".to_string(),
            "cmd" => "⌘".to_string(),
            "shift" => "⇧".to_string(),
            "alt" => "⌥".to_string(),
            other if other.len() == 1 => other.to_uppercase(),
            other => other.to_string(),
        })
        .collect()
}

fn keybind_box(text: String) -> impl Element {
    div()
        .flex_none()
        .p(px(5.))
        .bg(rgb(0x262626))
        .rounded_sm()
        .text_size(px(11.))
        .line_height(relative(1.0))
        .child(text)
}
