use std::time::Duration;

use gpui::{
    AsyncApp, Context, Entity, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Subscription, Task, WeakEntity, Window, div, hsla, px, rgb,
};

use crate::{
    ui::{error::view::ErrorView, launcher::LauncherView},
    utils::errors::SherlockError,
};

pub struct SherlockWorkspace {
    pub launcher: Entity<LauncherView>,
    pub error: Entity<ErrorView>,
    pub workspace: WorkspaceView,
    pub _subs: Vec<Subscription>,

    // Animation Stuff
    pub opacity: f32,
    pub transition_task: Option<Task<()>>,
    pub pending_focus: bool,
}

pub enum WorkspaceView {
    Launcher,
    Error,
}

impl Render for SherlockWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.pending_focus {
            self.pending_focus = false;
            match &self.workspace {
                WorkspaceView::Error => {
                    let handle = self.error.read(cx).focus_handle.clone();
                    window.focus(&handle, cx);
                }
                WorkspaceView::Launcher => {
                    let handle = self.launcher.read(cx).text_input.focus_handle(cx);
                    window.focus(&handle, cx);
                }
            }
        }

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
            .child(
                div()
                    .size_full()
                    .opacity(self.opacity)
                    .child(match self.workspace {
                        WorkspaceView::Launcher => self.launcher.clone().into_any_element(),
                        WorkspaceView::Error => self.error.clone().into_any_element(),
                    }),
            )
    }
}

impl SherlockWorkspace {
    pub fn transition_to(
        &mut self,
        target: WorkspaceView,
        duration_ms: u64,
        cx: &mut Context<Self>,
    ) {
        self.transition_task = None;
        self.transition_task = Some(cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                const STEPS: usize = 12;
                let frame_ms = (duration_ms / 2) / STEPS as u64;

                // fade out
                for i in (0..=STEPS).rev() {
                    let t = ease_in_out(i as f32 / STEPS as f32);
                    if this
                        .update(&mut cx, |view, cx| {
                            view.opacity = t;
                            cx.notify();
                        })
                        .is_err()
                    {
                        return;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(frame_ms))
                        .await;
                }

                // switch view at opacity 0
                if this
                    .update(&mut cx, |view, cx| {
                        view.workspace = target;
                        view.opacity = 0.0;
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }

                // pause at black
                cx.background_executor()
                    .timer(Duration::from_millis(frame_ms * 2))
                    .await;

                // fade in
                for i in 0..=STEPS {
                    let t = ease_in_out(i as f32 / STEPS as f32);
                    if this
                        .update(&mut cx, |view, cx| {
                            view.opacity = t;
                            cx.notify();
                        })
                        .is_err()
                    {
                        return;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(frame_ms))
                        .await;
                }

                let _ = this.update(&mut cx, |view, cx| {
                    view.pending_focus = true;
                    cx.notify();
                });
            }
        }));
    }
}

fn ease_in_out(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

pub enum LauncherErrorEvent {
    Push(SherlockError),
    ShowErrors,
}
impl gpui::EventEmitter<LauncherErrorEvent> for LauncherView {}
