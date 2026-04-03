use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use chrono::Local;
use gpui::{
    Animation, AnimationExt, AnyElement, App, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, SharedString, Styled, div, prelude::FluentBuilder, px, rgb,
};
use simd_json::prelude::ArrayTrait;
use suite_223b::{
    calendar::utils::{
        CalDavEvent,
        structs::{Attendee, EventFilter, Partstat},
    },
    protocol::{Request, Response, SocketData},
    tokio::{AsyncSizedMessage, SizedMessageObj},
};

use crate::{
    app::theme::ThemeData,
    launcher::{ExecMode, Launcher},
    loader::utils::ApplicationAction,
    sherlock_msg,
    ui::{
        launcher::{context_menu::ContextMenuAction, render::FIRST_RUN},
        widgets::{RenderableChildImpl, Selection},
    },
    utils::errors::{
        SherlockMessage,
        types::{SherlockErrorType, SocketAction},
    },
};

#[derive(Clone, Default, Debug)]
pub struct EventData {
    pub time: Option<SharedString>,
    pub event: Option<CalDavEvent>,
    pub color: Option<Hsla>,
    pub actions: Arc<[Arc<ContextMenuAction>]>,

    look_back: Duration,
    look_ahead: Duration,
    last_call: Option<Instant>,
    animation: Rc<Cell<AnimState>>,
}

#[derive(Clone, Copy, Default, Debug)]
enum AnimState {
    #[default]
    Inactive,
    Done,
    InProgress,
}

impl EventData {
    pub fn new(look_back: Duration, look_ahead: Duration) -> Self {
        Self {
            time: None,
            event: None,
            color: None,
            actions: Arc::new([]),

            look_back,
            look_ahead,
            last_call: None,
            ..Default::default()
        }
    }
    pub async fn update_async(&mut self) -> Result<(), SherlockMessage> {
        // debounce logic
        // causes freezes if not applied!!
        if let Some(last_call) = self.last_call {
            if last_call.elapsed() < Duration::from_secs(50) {
                return Ok(());
            }
        }
        self.last_call = Some(Instant::now());

        let mut stream = tokio::net::UnixStream::connect(SocketData::SOCKET_ADDR)
            .await
            .map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::SocketError(
                        SocketAction::Connect,
                        SocketData::SOCKET_ADDR.into()
                    ),
                    e
                )
            })?;

        let config = bincode::config::standard();
        let req = Request::Event(EventFilter::Nearby {
            look_back: self.look_back,
            look_ahead: self.look_ahead,
        });
        let req_obj = SizedMessageObj::from_struct(&req).map_err(|e| {
            sherlock_msg!(Warning, SherlockErrorType::SerializationError, e.message)
        })?;

        stream.write_sized(req_obj).await.map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::SocketError(SocketAction::Write, SocketData::SOCKET_ADDR.into()),
                e.message
            )
        })?;

        let resp_bin = stream.read_sized().await.map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::SocketError(SocketAction::Read, SocketData::SOCKET_ADDR.into()),
                e.message
            )
        })?;
        let (resp, _): (Response, _) = bincode::serde::decode_from_slice(&resp_bin, config)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))?;

        if let Response::Events(mut events) = resp {
            let now = Local::now();

            events.sort_by(|a, b| {
                let a_start = a
                    .start_utc()
                    .map(|t| t.with_timezone(&Local))
                    .unwrap_or(now);
                let b_start = b
                    .start_utc()
                    .map(|t| t.with_timezone(&Local))
                    .unwrap_or(now);
                let a_end = a.end_utc().map(|t| t.with_timezone(&Local)).unwrap_or(now);
                let b_end = b.end_utc().map(|t| t.with_timezone(&Local)).unwrap_or(now);

                let a_is_active = now >= a_start && now < a_end;
                let b_is_active = now >= b_start && now < b_end;
                let a_is_upcoming = a_start > now;
                let b_is_upcoming = b_start > now;

                match (a_is_active, b_is_active) {
                    // both active: prefer the one ending soonest (most immediately relevant)
                    (true, true) => a_end.cmp(&b_end),

                    // one active, one not: active always wins
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,

                    // neither one is active: upcoming beats past
                    (false, false) => match (a_is_upcoming, b_is_upcoming) {
                        // both upcoming: soonest first
                        (true, true) => a_start.cmp(&b_start),

                        // one upcoming, one past: upcoming wins
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,

                        // both past: most recently ended first (least stale)
                        (false, false) => b_end.cmp(&a_end),
                    },
                }
            });

            self.event = events.into_iter().next();
            self.time = self
                .event
                .as_ref()
                .and_then(|e| e.start_utc())
                .map(|utc_dt| {
                    utc_dt
                        .with_timezone(&Local)
                        .format("%H:%M")
                        .to_string()
                        .into()
                });
            self.color = self
                .event
                .as_ref()
                .and_then(|e| e.calendar_info.color.as_deref())
                .map(hex_to_u32)
                .map(rgb)
                .map(|s| s.into());
        }
        Ok(())
    }
}
impl<'a> RenderableChildImpl<'a> for EventData {
    fn render(
        &self,
        _launcher: &Arc<Launcher>,
        selection: Selection,
        theme: Arc<ThemeData>,
        _cx: &mut App,
    ) -> AnyElement {
        let Some(ref event) = self.event else {
            return div().into_any_element();
        };

        if FIRST_RUN.load(Ordering::Relaxed) {
            self.animation.set(AnimState::Inactive);
        }

        let accent_color = self.color.unwrap_or(theme.bg_idle);
        div()
            .relative()
            .group("event-card")
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_start()
            .border_1()
            .rounded_md()
            .when(!selection.is_selected, |this| {
                this.border_color(theme.border_idle)
            })
            .child(
                div()
                    .size_full()
                    .flex()
                    .gap_5()
                    .px_2()
                    .items_center()
                    .child(div().size(px(8.0)).rounded_full().bg(accent_color))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .child(
                                // title and loc
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_family(theme.font_family.clone())
                                            .text_color(theme.primary_text)
                                            .child(event.title.clone()),
                                    )
                                    .when_some(event.location.clone(), |this, loc| {
                                        this.flex()
                                            .child(
                                                div()
                                                    .rounded_full()
                                                    .bg(theme.secondary_text)
                                                    .size(px(5.)),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .font_family(theme.font_family.clone())
                                                    .text_color(theme.secondary_text)
                                                    .child(loc),
                                            )
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .font_family(theme.font_family.clone())
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.secondary_text)
                                            .children(self.time.as_ref().map(|t| t.clone())),
                                    )
                                    .child(
                                        div()
                                            .px_1()
                                            .py_0()
                                            .rounded_sm()
                                            .bg(theme.bg_badge)
                                            .text_size(px(10.0))
                                            .font_family(theme.font_family.clone())
                                            .text_color(theme.secondary_text)
                                            .child(event.calendar_info.name.clone()),
                                    ),
                            ),
                    ),
            )
            .when(
                !event.attendees.is_empty()
                    && (matches!(
                        self.animation.get(),
                        AnimState::InProgress | AnimState::Done
                    ) || selection.is_selected),
                |this| {
                    this.child(
                        div()
                            .px_2()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .border_t_1()
                            .border_color(theme.border_idle)
                            .child(
                                // A "Section Header" that looks like a tag
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_family(theme.font_family.clone())
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.secondary_text)
                                            .child("ATTENDEES"),
                                    )
                                    .child(
                                        div()
                                            .px_1()
                                            .rounded_sm()
                                            .bg(theme.bg_selected)
                                            .text_size(px(9.))
                                            .font_family(theme.font_family.clone())
                                            .text_color(theme.primary_text)
                                            .child(event.attendees.len().to_string()),
                                    ),
                            )
                            .child(
                                div().flex().flex_col().gap_1_5().children(
                                    event
                                        .attendees
                                        .iter()
                                        .take(8)
                                        .map(|att| render_attendee(att, &theme)),
                                ),
                            )
                            .with_animation(
                                if selection.is_selected {
                                    "attendee-reveal"
                                } else {
                                    "attendee-veal"
                                },
                                Animation::new(Duration::from_millis(200)).with_easing(SMOOTH_EASE),
                                {
                                    let anim_state = Rc::clone(&self.animation);
                                    let selection = selection.is_selected;
                                    move |this, delta| {
                                        let is_done = delta == 1.0;
                                        if is_done {
                                            if selection {
                                                anim_state.set(AnimState::Done);
                                            } else {
                                                anim_state.set(AnimState::Inactive);
                                            }
                                        } else {
                                            anim_state.set(AnimState::InProgress);
                                        }
                                        let delta = if selection { delta } else { 1.0 - delta };
                                        this.py(px(12. * delta))
                                            .mt(px(20. * delta))
                                            .opacity(delta)
                                            .max_h(px(delta * 200.))
                                            .occlude()
                                    }
                                },
                            ),
                    )
                },
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
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        ""
    }
    #[inline(always)]
    fn actions(
        &self,
        launcher: &Arc<Launcher>,
        _cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        let event = self.event.as_ref()?;
        let meeting = event.meeting.as_ref();
        let extra = launcher.add_actions.as_ref();

        // early return if only actions apply
        if meeting.is_none() && extra.map_or(true, |e| e.is_empty()) {
            return Some(Arc::clone(&self.actions));
        }

        let mut cap = self.actions.len();
        if meeting.is_some() {
            cap += 1;
        }
        if let Some(e) = extra {
            cap += e.len();
        }

        let mut actions = Vec::with_capacity(cap);

        if let Some(url) = meeting.map(|m| m.url()) {
            actions.push(Arc::new(ContextMenuAction::App(
                ApplicationAction::new("inner.join_meeting")
                    .name("Join Meeting")
                    .icon_name("call-start")
                    .exec(url.to_string()),
            )));
        }

        actions.extend(self.actions.iter().cloned());
        if let Some(extra_actions) = extra {
            actions.extend(extra_actions.iter().cloned());
        }

        Some(Arc::from(actions))
    }
    #[inline(always)]
    fn has_actions(&self, _cx: &mut App) -> bool {
        self.event.is_some()
    }
    #[inline(always)]
    fn based_show(&self, _keyword: &str) -> Option<bool> {
        Some(self.event.is_some())
    }
}

#[inline(always)]
fn hex_to_u32(hex: &str) -> u32 {
    let cleaned = hex.strip_prefix('#').unwrap_or(hex);
    u32::from_str_radix(cleaned, 16).unwrap_or(0)
}

fn render_attendee(attendee: &Attendee, theme: &ThemeData) -> impl IntoElement {
    let name = attendee.display_name.as_deref();
    let email = attendee.email.as_deref();

    let color = match attendee.partstat {
        Some(Partstat::Accepted) => theme.color_succ,
        Some(Partstat::Tentative) => theme.color_warn,
        Some(Partstat::Declined) => theme.color_err,
        _ => theme.secondary_text,
    };

    div()
        .flex()
        .flex_row()
        .justify_start()
        .items_center()
        .gap_2()
        .child(div().size(px(5.)).rounded_full().bg(color))
        .child(
            div()
                .flex()
                .flex_row()
                .items_baseline()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_family(theme.font_family.clone())
                        .text_color(theme.primary_text)
                        .child(name.unwrap_or(email.unwrap_or("Unknown")).to_string()),
                )
                .when(name.is_some() && email.is_some(), |this| {
                    this.child(
                        div()
                            .text_size(px(12.0))
                            .font_family(theme.font_family.clone())
                            .text_color(theme.secondary_text.opacity(0.7))
                            .child(format!("({})", email.unwrap())),
                    )
                }),
        )
}

const SMOOTH_EASE: fn(f32) -> f32 = |t| {
    // This is a common "Ease-In-Out-Cubic" curve
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - ((-2.0 * t + 2.0).powi(3)) / 2.0
    }
};
