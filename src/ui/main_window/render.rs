use std::sync::Arc;

use gpui::{
    AnyElement, Context, Element, Focusable, FontWeight, Image, ImageSource, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
    div, hsla, img, list, px, relative, rgb,
};

use crate::{
    CONTEXT_MENU_BIND,
    launcher::children::{RenderableChild, RenderableChildDelegate},
    ui::{UIFunction, main_window::SherlockMainWindow},
    utils::config::ConfigGuard,
};

impl Render for SherlockMainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak_self = cx.entity().downgrade();
        div()
            .id("sherlock")
            .track_focus(&self.focus_handle(cx))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x0F0F0F))
            .border_2()
            .border_color(hsla(0., 0., 0.1882, 1.0))
            .rounded(px(5.))
            .shadow_xl()
            .overflow_hidden()
            .on_action(cx.listener(Self::focus_next))
            .on_action(cx.listener(Self::focus_prev))
            .on_action(cx.listener(Self::next_var))
            .on_action(cx.listener(Self::prev_var))
            .on_action(cx.listener(Self::execute))
            .on_action(cx.listener(Self::quit))
            .on_action(cx.listener(Self::open_context))
            .child(
                // search bar
                div()
                    .flex()
                    .flex_row()
                    .w_full()
                    .items_center()
                    .px_4()
                    .py(px(4.))
                    .gap_3()
                    .child(div().text_color(rgb(0x888888)).child(""))
                    .child(div().w_auto().child(self.text_input.clone()))
                    .children(self.variable_input.iter().cloned())
                    .border_b_2()
                    .border_color(hsla(0., 0., 0.1882, 1.0)),
            )
            .child(
                div()
                    .px(px(14.))
                    .py(px(4.))
                    .text_size(px(14.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x2e2e2e))
                    .child(self.mode.display_str()),
            )
            .child(
                div()
                    .id("results-container")
                    .flex_1()
                    .min_h_0()
                    .px(px(10.))
                    .child(
                        list(self.list_state.clone(), move |idx, _win, cx| {
                            // 1. Upgrade and Read
                            let entity = weak_self.upgrade();
                            if entity.is_none() {
                                return div().into_any_element();
                            }
                            let state = entity.unwrap().read(cx);

                            // 2. Bounds Check - If this fails, we return an empty div to satisfy AnyElement
                            let data_idx = match state.filtered_indices.get(idx) {
                                Some(&i) => i,
                                None => return div().into_any_element(),
                            };

                            let data_guard = state.data.read(cx);
                            let child = match data_guard.get(data_idx) {
                                Some(c) => c,
                                None => return div().into_any_element(),
                            };

                            state.render_list_item(&child, idx)
                        })
                        .size_full(),
                    )
                    .child(if let Some(active) = self.context_idx {
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
                                .children(self.context_actions.iter().enumerate().map(
                                    |(i, child)| {
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
                                                if is_selected && self.context_idx.is_some() {
                                                    s
                                                } else {
                                                    s.bg(hsla(0., 0., 0.12, 1.0))
                                                }
                                            })
                                            .child(if let Some(icon) = child.icon.as_ref() {
                                                img(Arc::clone(&icon))
                                                    .size(px(16.))
                                                    .into_any_element()
                                            } else {
                                                img(ImageSource::Image(Arc::new(Image::empty())))
                                                    .size(px(16.))
                                                    .into_any_element()
                                            })
                                            .child(child.name.as_ref().unwrap().clone())
                                    },
                                )),
                        )
                    } else {
                        div()
                    }),
            )
            .child(
                // statusbar
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
                    .child(div().flex_1())
                    .child({
                        let guard = self.data.read(cx);
                        if let Some(true) = self
                            .filtered_indices
                            .get(self.selected_index)
                            .and_then(|i| guard.get(*i))
                            .and_then(RenderableChild::actions)
                            .map(|a| !a.is_empty())
                        {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .child(div().mr_1().child(SharedString::from("Additional Actions")))
                                .children(
                                    get_context_key_parts().into_iter().map(|p| keybind_box(p)),
                                )
                        } else {
                            div()
                        }
                    }),
            )
    }
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

impl SherlockMainWindow {
    fn render_list_item(&self, ad: &RenderableChild, idx: usize) -> AnyElement {
        let is_selected = self.selected_index == idx;
        div()
            .id(("keystroke", idx))
            .w_full()
            .on_click(move |_, _, _| {
                println!("Clicked item {}", idx);
            })
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
                        if is_selected || self.context_idx.is_some() {
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
