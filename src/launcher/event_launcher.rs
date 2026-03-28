use serde::Deserialize;
use suite_223b::calendar::utils::MeetingProvider;

use crate::{
    ensure_func,
    launcher::{
        LauncherProvider,
        children::{RenderableChild, event_data::EventData},
        variant_type::{InnerFunction, LauncherType},
    },
    loader::utils::RawLauncher,
    sherlock_msg,
    utils::{
        command_launch,
        config::ConfigGuard,
        errors::{SherlockMessage, types::SherlockErrorType},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, strum::VariantNames, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum EventLauncherFunctions {
    HardRefresh,
    JoinMeeting,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EventLauncher {}

impl LauncherProvider for EventLauncher {
    fn parse(_raw: &RawLauncher) -> LauncherType {
        LauncherType::Event(Self {})
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<super::Launcher>,
        _ctx: &crate::loader::LoadContext,
        _opts: std::sync::Arc<serde_json::Value>,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        Ok(vec![RenderableChild::EventLike {
            launcher,
            inner: EventData::default(),
        }])
    }
    fn execute_function(
        &self,
        func: super::variant_type::InnerFunction,
        child: &RenderableChild,
    ) -> Result<bool, SherlockMessage> {
        let func = ensure_func!(func, InnerFunction::Event);

        let RenderableChild::EventLike { inner, .. } = child else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::Unreachable,
                format!("Tried to unpack Event tile but received: {:?}", child)
            ));
        };

        match func {
            EventLauncherFunctions::JoinMeeting => {
                if let Some(meeting) = inner.event.as_ref().and_then(|e| e.meeting.as_ref()) {
                    match meeting.provider {
                        MeetingProvider::MicrosoftTeams => {
                            return teamslaunch(&meeting.url).map(|_| true);
                        }
                    }
                }
            }
            EventLauncherFunctions::HardRefresh => {
                println!("hard refresh");
            }
        }

        Ok(false)
    }
}

pub fn teamslaunch(meeting_url: &str) -> Result<(), SherlockMessage> {
    let command =
        ConfigGuard::read().map(|c| c.default_apps.teams.replace("{meeting_url}", meeting_url))?;
    command_launch::spawn_detached(&command, "", &[])
}
