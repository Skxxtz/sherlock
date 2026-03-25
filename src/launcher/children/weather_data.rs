use chrono::Local;
use gpui::{
    AnyElement, Image, ImageSource, IntoElement, ParentElement, Styled, div, img, linear_gradient,
    px,
};
use std::sync::Arc;

use crate::{
    launcher::{ExecMode, Launcher, children::RenderableChildImpl, weather_launcher::WeatherData},
    loader::utils::ContextMenuAction,
};

impl<'a> RenderableChildImpl<'a> for WeatherData {
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<ExecMode> {
        None
    }
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    fn search(&self, _launcher: &Arc<Launcher>) -> &'a str {
        ""
    }
    fn render(&self, _launcher: &Arc<Launcher>, _is_selected: bool) -> AnyElement {
        let now = Local::now().time();
        div()
            .px_4()
            .py_2()
            .rounded_md()
            .bg({
                let (p1, p2) = self.css.background(now, self.sunset, self.sunrise);
                linear_gradient(90., p1, p2)
            })
            .text_color(self.css.color(now, self.sunset, self.sunrise))
            .flex_col()
            .gap_5()
            .items_center()
            .text_size(px(12.0))
            .child(self.format_str.clone())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_5()
                    .child(if let Some(icon) = self.icon.as_ref() {
                        img(Arc::clone(&icon)).size(px(48.))
                    } else {
                        img(ImageSource::Image(Arc::new(Image::empty()))).size(px(24.))
                    })
                    .child(div().text_size(px(40.0)).child(self.temperature.clone())),
            )
            .into_any_element()
    }
    fn actions(&self) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        None
    }
}
