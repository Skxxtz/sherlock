use serde::Deserialize;
use serde::Serialize;
use zbus::zvariant::DeserializeDict;
use zbus::zvariant::Type;

#[derive(DeserializeDict, Type, Debug, Clone)]
#[zvariant(signature = "a{sv}")]
#[allow(unused)]
pub struct MprisData {
    #[zvariant(rename = "PlaybackStatus")]
    pub playback_status: String,

    #[zvariant(rename = "Metadata")]
    pub metadata: MetaData,
}
#[derive(DeserializeDict, Type, Debug, Clone)]
#[zvariant(signature = "a{sv}")]
#[allow(unused)]
pub struct MetaData {
    #[zvariant(rename = "xesam:title")]
    pub title: String,

    #[zvariant(rename = "xesam:album")]
    pub album: String,

    #[zvariant(rename = "xesam:artist")]
    pub artists: Vec<String>,

    #[zvariant(rename = "xesam:url")]
    pub url: String,

    #[zvariant(rename = "mpris:artUrl")]
    pub art: String,
}

pub fn to_title_case(input_str: &str) -> String {
    let mut result = String::with_capacity(input_str.len());
    let mut cap_next = true;
    for c in input_str.chars() {
        if c.is_whitespace() {
            cap_next = true;
            result.push(c);
        } else if cap_next {
            for up in c.to_uppercase() {
                result.push(up)
            }
            cap_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Serialize)]
pub enum HomeType {
    OnlyHome,
    Home,
    Search,
}
impl Default for HomeType {
    fn default() -> Self {
        HomeType::Search
    }
}
