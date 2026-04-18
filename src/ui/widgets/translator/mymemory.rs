use gpui::SharedString;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MyMemoryResponse {
    #[serde(rename = "responseData")]
    pub response_data: ResponseData,
}

#[derive(Debug, Deserialize)]
pub struct ResponseData {
    #[serde(rename = "translatedText")]
    pub translated_text: SharedString,
}
