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
