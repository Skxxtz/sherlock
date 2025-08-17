use std::collections::HashMap;

use super::commandlaunch::command_launch;
use crate::utils::{
    config::{ConfigGuard, ConstantDefaults},
    errors::SherlockError,
};

pub fn websearch(engine: &str, query: &str, browser: Option<&str>) -> Result<(), SherlockError> {
    let engines: HashMap<&str, &str> = HashMap::from([
        ("google", "https://www.google.com/search?q={keyword}"),
        ("bing", "https://www.bing.com/search?q={keyword}"),
        ("duckduckgo", "https://duckduckgo.com/?q={keyword}"),
        ("yahoo", "https://search.yahoo.com/search?p={keyword}"),
        ("baidu", "https://www.baidu.com/s?wd={keyword}"),
        ("yandex", "https://yandex.com/search/?text={keyword}"),
        ("ask", "https://www.ask.com/web?q={keyword}"),
        ("ecosia", "https://www.ecosia.org/search?q={keyword}"),
        ("qwant", "https://www.qwant.com/?q={keyword}"),
        (
            "startpage",
            "https://www.startpage.com/sp/search?q={keyword}",
        ),
        ("plain", "{keyword}"),
    ]);
    let url_template = if let Some(url) = engines.get(engine) {
        url
    } else {
        engine
    };

    let mut browser = match browser {
        Some(b) => b.to_string(),
        None => {
            let c = ConfigGuard::read()?;
            c.default_apps
                .browser
                .clone()
                .unwrap_or(ConstantDefaults::browser()?)
        }
    };

    let url = url_template.replace("{keyword}", &query.replace(" ", "+"));
    let command = if browser.contains("%u") {
        browser.replace("%u", &format!(r#" "{}""#, url))
    } else {
        browser.push_str(&format!(r#" "{}""#, url));
        browser
    };
    command_launch(&command, "")
}
