use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Local;
use gpui::{
    AnyElement, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, SharedString,
    Styled, div, hsla, prelude::FluentBuilder, px, rgb,
};
use simd_json::prelude::{ArrayMut, ArrayTrait};
use suite_223b::{
    calendar::utils::{
        CalDavEvent,
        structs::{Attendee, EventFilter, Partstat},
    },
    protocol::{Request, Response, SocketData},
    tokio::{AsyncSizedMessage, SizedMessageObj},
};

use crate::{
    app::ActiveTheme,
    launcher::{
        ExecMode, Launcher,
        children::{RenderableChildImpl, Selection},
    },
    loader::utils::ApplicationAction,
    sherlock_msg,
    ui::launcher::context_menu::ContextMenuAction,
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

                let a_is_active = now >= a_start && now <= a_end;
                let b_is_active = now >= b_start && now <= b_end;

                match (a_is_active, b_is_active) {
                    (true, true) => b_start.cmp(&a_start),
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    (false, false) => a_start.cmp(&b_start),
                }
            });

            // Now the "Best" event is at index 0
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
        theme: &ActiveTheme,
    ) -> AnyElement {
        let Some(ref event) = self.event else {
            return div().into_any_element();
        };

        let accent_color = self.color.unwrap_or(rgb(0xff453a).into());
        div()
            .group("event-card")
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .flex_col()
            .gap_5()
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
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.secondary_text)
                                            .children(self.time.as_ref().map(|t| t.clone())),
                                    )
                                    .child(
                                        div()
                                            .px_1()
                                            .py_0()
                                            .rounded_sm()
                                            .bg(hsla(0.0, 0.0, 1.0, 0.08))
                                            .text_size(px(10.0))
                                            .text_color(theme.secondary_text)
                                            .child(event.calendar_info.name.clone()),
                                    ),
                            ),
                    ),
            )
            .when(
                selection.is_selected && !event.attendees.is_empty(),
                |this| {
                    this.child(
                        div()
                            .px_2()
                            .mb_3() // Add space between the main row and details
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                // A "Section Header" that looks like a tag
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(10.))
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
                                            .text_color(theme.primary_text)
                                            .child(event.attendees.len().to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1_5()
                                    .pl_1() // Slight offset for hierarchy
                                    .children(
                                        event
                                            .attendees
                                            .iter()
                                            .take(8) // Safety: Don't explode the height if it's a 50-person meeting
                                            .map(|att| render_attendee(att, theme)),
                                    ),
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
    fn actions(&self) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        let event = self.event.as_ref()?; // Guard: if no event, no actions.

        // Check if we actually need to build a dynamic list.
        // If there's no meeting, we can just return our existing base actions Arc.
        let meeting = event.meeting.as_ref();

        if meeting.is_none() {
            return Some(Arc::clone(&self.actions));
        }

        // We have a meeting, so we must allocate a new Vec to merge lists.
        let meeting = meeting.unwrap();
        let mut actions = Vec::with_capacity(self.actions.len() + 1);

        // 1. Push the Priority Dynamic Action
        let url = meeting.url.clone();
        actions.push(Arc::new(ContextMenuAction::App(
            ApplicationAction::new("inner.join_meeting")
                .name("Join Meeting")
                .icon_name("call-start")
                .exec(url),
        )));

        actions.extend(self.actions.iter().cloned());

        Some(Arc::from(actions))
    }
    #[inline(always)]
    fn has_actions(&self) -> bool {
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

fn render_attendee(attendee: &Attendee, theme: &ActiveTheme) -> impl IntoElement {
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
                        .text_color(theme.primary_text)
                        .child(name.unwrap_or(email.unwrap_or("Unknown")).to_string()),
                )
                .when(name.is_some() && email.is_some(), |this| {
                    this.child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.secondary_text.opacity(0.7))
                            .child(format!("({})", email.unwrap())),
                    )
                }),
        )
}
