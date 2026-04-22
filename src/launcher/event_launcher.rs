use std::time::Duration;

use gpui::AppContext;
use serde::Deserialize;

use crate::{
    ensure_func,
    launcher::{
        LauncherProvider,
        variant_type::{InnerFunction, LauncherType},
    },
    loader::utils::RawLauncher,
    sherlock_msg,
    ui::widgets::{RenderableChild, event::EventData},
    utils::{
        command_launch::{mime_lookup, spawn_detached},
        errors::{SherlockMessage, types::SherlockErrorType},
        websearch::websearch,
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
        opts: std::sync::Arc<serde_json::Value>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let look_back_raw = opts
            .get("look_back")
            .and_then(|v| v.as_str())
            .unwrap_or("10mins");

        let look_ahead_raw = opts
            .get("look_ahead")
            .and_then(|v| v.as_str())
            .unwrap_or("1h");

        let look_back = parse_dynamic_time(look_back_raw).unwrap_or(Duration::from_hours(6));
        let look_ahead = parse_dynamic_time(look_ahead_raw).unwrap_or(Duration::from_hours(4));

        Ok(vec![RenderableChild::Event {
            launcher,
            inner: Box::new(EventData::new(look_back, look_ahead)),
        }])
    }
    fn execute_function<C: AppContext>(
        &self,
        func: super::variant_type::InnerFunction,
        child: &RenderableChild,
        _cx: &mut C,
    ) -> Result<bool, SherlockMessage> {
        let func = ensure_func!(func, InnerFunction::Event);

        let RenderableChild::Event { inner, .. } = child else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::Unreachable,
                format!("Tried to unpack Event tile but received: {:?}", child)
            ));
        };

        match func {
            EventLauncherFunctions::JoinMeeting => {
                if let Some(meeting) = inner.event.as_ref().and_then(|e| e.meeting.as_ref()) {
                    if let Some(command) = mime_lookup(meeting.protocol_prefix()) {
                        let url = meeting.mime_url();
                        spawn_detached(&command.replace("%u", &url), "", &[])?;
                    } else {
                        let url = meeting.https_url();
                        websearch("plain", &url, None, &[])?;
                    }
                    return Ok(true);
                }
            }
            EventLauncherFunctions::HardRefresh => {
                println!("hard refresh");
            }
        }

        Ok(false)
    }
}

pub fn parse_dynamic_time(input: &str) -> Option<Duration> {
    let input = input.trim().to_lowercase();

    // 1. Find the first character that isn't a number (could be a space or a letter)
    let first_non_digit = input.find(|c: char| !c.is_numeric())?;

    // 2. Split the string: "4 hrs" -> ("4", " hrs")
    let (numeric_part, unit_part) = input.split_at(first_non_digit);

    // 3. Trim both and parse the number
    let amount: u64 = numeric_part.trim().parse().ok()?;

    // 4. The match handles the unit, ignoring any leading/trailing spaces
    match unit_part.trim() {
        "m" | "min" | "mins" | "minute" | "minutes" => Some(Duration::from_secs(amount * 60)),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(Duration::from_secs(amount * 3600)),
        "d" | "day" | "days" => Some(Duration::from_secs(amount * 86400)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_parse_dynamic_time() {
        // Test: Basic units without spaces
        assert_eq!(parse_dynamic_time("4m"), Some(Duration::from_secs(4 * 60)));
        assert_eq!(
            parse_dynamic_time("2h"),
            Some(Duration::from_secs(2 * 3600))
        );
        assert_eq!(
            parse_dynamic_time("1d"),
            Some(Duration::from_secs(1 * 86400))
        );

        // Test: Units with spaces (Your specific requirement)
        assert_eq!(
            parse_dynamic_time("4 min"),
            Some(Duration::from_secs(4 * 60))
        );
        assert_eq!(
            parse_dynamic_time("4 hrs"),
            Some(Duration::from_secs(4 * 3600))
        );
        assert_eq!(
            parse_dynamic_time("1 day"),
            Some(Duration::from_secs(1 * 86400))
        );

        // Test: Plurals and variations
        assert_eq!(
            parse_dynamic_time("10minutes"),
            Some(Duration::from_secs(10 * 60))
        );
        assert_eq!(
            parse_dynamic_time("5hour"),
            Some(Duration::from_secs(5 * 3600))
        );
        assert_eq!(
            parse_dynamic_time("3 days"),
            Some(Duration::from_secs(3 * 86400))
        );

        // Test: Case sensitivity and extra whitespace
        assert_eq!(
            parse_dynamic_time("  6HRS  "),
            Some(Duration::from_secs(6 * 3600))
        );
        assert_eq!(
            parse_dynamic_time("12   min"),
            Some(Duration::from_secs(12 * 60))
        );

        // Test: Sad paths (Invalid inputs)
        assert_eq!(parse_dynamic_time("now"), None);
        assert_eq!(parse_dynamic_time("5z"), None);
        assert_eq!(parse_dynamic_time("5 5 h"), None);
        assert_eq!(parse_dynamic_time(""), None);
        assert_eq!(parse_dynamic_time("hours 5"), None);
        assert_eq!(parse_dynamic_time("+ 5 hours"), None);
    }
}
