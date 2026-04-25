use chrono::Local;
use gpui::{
    Animation, AnimationExt, AnyElement, App, Image, ImageSource, IntoElement, ParentElement,
    Styled, div, img, linear_gradient, px,
};
use std::{sync::Arc, time::Duration};

use crate::{
    app::theme::ThemeData,
    launcher::{ExecMode, Launcher, weather_launcher::WeatherData},
    ui::widgets::{RenderableChildImpl, Selection},
};

impl<'a> RenderableChildImpl<'a> for WeatherData {
    const HANDLES_BORDERS: bool = true;
    fn render(
        &self,
        _launcher: &Arc<Launcher>,
        _selection: Selection,
        theme: Arc<ThemeData>,
        _cx: &mut App,
    ) -> AnyElement {
        let now = Local::now().time();
        let is_init = self.init;
        div()
            .rounded_md()
            .bg({
                let (p1, p2) = self.css.background(now, self.sunset, self.sunrise);
                linear_gradient(90., p1, p2)
            })
            .child(
                div()
                    .px_4()
                    .py_2()
                    .text_color(self.css.color(now, self.sunset, self.sunrise))
                    .flex_col()
                    .gap_5()
                    .items_center()
                    .text_size(px(12.0))
                    .font_family(theme.font_family.clone())
                    .child(self.format_str.clone())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_5()
                            .child(if let Some(icon) = self.icon.as_ref() {
                                img(Arc::clone(icon)).size(px(48.))
                            } else {
                                img(ImageSource::Image(Arc::new(Image::empty()))).size(px(24.))
                            })
                            .child(
                                div()
                                    .text_size(px(40.0))
                                    .font_family(theme.font_family.clone())
                                    .child(self.temperature.clone()),
                            )
                            .with_animation(
                                "weather_fade_in",
                                Animation::new(Duration::from_millis(200))
                                    .with_easing(|t| t * t * (3.0 - 2.0 * t)),
                                move |this, frac| {
                                    let opacity = if is_init { frac } else { 0.0 };
                                    this.opacity(opacity.clamp(0.0, 1.0))
                                },
                            ),
                    ),
            )
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<ExecMode> {
        None
    }
    #[inline(always)]
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    #[inline(always)]
    fn search(&self, _launcher: &Arc<Launcher>) -> &'a str {
        ""
    }
}
