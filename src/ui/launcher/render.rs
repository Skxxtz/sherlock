use std::sync::Arc;

use gpui::{
    AnyElement, Context, Element, FontWeight, InteractiveElement, IntoElement, MouseDownEvent,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window, div, hsla,
    list, prelude::FluentBuilder, px, relative, rgb,
};

use crate::{
    CONTEXT_MENU_BIND,
    app::{ActiveTheme, ThemeData},
    launcher::children::{LauncherValues, RenderableChild, RenderableChildDelegate, Selection},
    ui::{
        UIFunction,
        launcher::{LauncherView, context_menu::ContextMenuAction, views::EntityStyle},
    },
    utils::config::ConfigGuard,
};

impl Render for LauncherView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_binds = self
            .navigation
            .selected_item(cx)
            .and_then(|i| i.launcher_type().binds());
        div()
            .id("sherlock")
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x0F0F0F))
            .border_2()
            .border_color(hsla(0., 0., 0.1882, 1.0))
            .rounded(px(5.))
            .shadow_xl()
            .overflow_hidden()
            .on_action(cx.listener(Self::selection_up))
            .on_action(cx.listener(Self::selection_down))
            .on_action(cx.listener(Self::selection_left))
            .on_action(cx.listener(Self::selection_right))
            .on_action(cx.listener(Self::next_var))
            .on_action(cx.listener(Self::prev_var))
            .on_action(cx.listener(Self::execute_listener))
            .on_action(cx.listener(Self::quit))
            .on_action(cx.listener(Self::open_context))
            .on_key_up(cx.listener(move |this, ev: &gpui::KeyUpEvent, win, cx| {
                if let Some(binds) = &selected_binds {
                    if let Some(pressed) = binds.iter().find(|bind| bind.matches(&ev.keystroke)) {
                        this.execute_inner_function(pressed.get_exec(), win, cx);
                    }
                }
            }))
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
            .with_model(cx, |mdl| (mdl.filtered_indices(), mdl.data()));
        let theme = cx.global::<ActiveTheme>().0.clone();
        let sidebar = self.navigation.with_selected_item(cx, |selected_item| {
            selected_item.and_then(|s| s.sidebar(theme.clone()))
        });
        let EntityStyle::Row {
            state,
            selected_index,
        } = &self.navigation.current().style
        else {
            return div().id("results-container");
        };

        let theme = cx.global::<ActiveTheme>().0.clone();
        div()
            .id("results-container")
            .relative()
            .flex_1()
            .min_h_0()
            .px(px(10.))
            .gap(px(10.))
            .flex()
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

                    Self::render_list_item(
                        &child,
                        Selection::new(data_idx, idx == selected_idx),
                        theme.clone(),
                    )
                })
                .size_full()
            })
            .children(sidebar)
            .child(self.render_context_menu())
    }

    fn render_result_grid(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (indices, data) = self
            .navigation
            .with_model(cx, |mdl| (mdl.filtered_indices(), mdl.data()));

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
                        let theme = cx.global::<ActiveTheme>().0.clone();
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
                                                        Selection::new(
                                                            data_idx,
                                                            item_idx == selected_idx,
                                                        ),
                                                        theme.clone(),
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
            .child(self.render_context_menu())
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
                        match child.as_ref() {
                            ContextMenuAction::App(_) => {
                                child.render_row(is_selected).into_any_element()
                            }
                            ContextMenuAction::Fn(_) => {
                                child.render_row(is_selected).into_any_element()
                            }
                            ContextMenuAction::Emoji(_) => {
                                child.render_col(is_selected).into_any_element()
                            }
                        }
                    })),
            )
        } else {
            div()
        }
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let message_count = self.navigation.message_count(cx);
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
            .when(message_count > 0, |this| {
                this.child(self.render_error_indicator(message_count, cx))
            })
            .child(div().flex_1())
            .child(self.render_context_hint(cx))
    }

    fn render_error_indicator(&self, count: usize, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .ml(px(10.))
            .h_full()
            .flex()
            .items_center()
            .gap(px(2.))
            .p(px(5.))
            .cursor_pointer()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.text_input.update(cx, |this, _| this.reset());
                    this.navigation.set_messages_active();
                    this.filter_and_sort(cx);
                    cx.notify();
                }),
            )
            .when(count > 0, |this| {
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
                        .text_xs()
                        .child(count.to_string()),
                )
            })
    }

    fn render_context_hint(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        if self.navigation.selected_index().is_some() {
            return div();
        };

        if self.has_actions {
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
        selection: Selection,
        theme: Arc<ThemeData>,
    ) -> AnyElement {
        div()
            .id(selection.data_idx)
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
                    .border_1()
                    .bg(theme.bg_idle)
                    .when(selection.is_selected, |this| {
                        this.bg(theme.bg_selected)
                            .border_color(theme.border_selected)
                    })
                    .child(ad.render(selection, theme)),
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
